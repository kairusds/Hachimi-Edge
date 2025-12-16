use std::{fs, io::{Read, Write}, path::{Path, PathBuf}, sync::{atomic::{self, AtomicUsize, AtomicBool}, mpsc, Arc, Mutex}, thread, cmp::max};

use arc_swap::ArcSwap;
use fnv::FnvHashMap;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use size::Size;
use thread_priority::{ThreadBuilderExt, ThreadPriority};

use crate::core::game::Region;
use super::{gui::SimpleYesNoDialog, hachimi::LocalizedData, http::{self, AsyncRequest}, utils, Error, Gui, Hachimi};
use once_cell::sync::Lazy;

#[derive(Deserialize)]
pub struct RepoInfo {
    pub name: String,
    pub index: String,
    pub short_desc: Option<String>,
    #[serde(default)]
    pub region: Region
}

pub fn new_meta_index_request() -> AsyncRequest<Vec<RepoInfo>> {
    let meta_index_url = &Hachimi::instance().config.load().meta_index_url;
    AsyncRequest::with_json_response(ureq::get(meta_index_url))
}

#[derive(Deserialize)]
struct RepoIndex {
    base_url: String,
    zip_url: String,
    zip_dir: String,
    files: Vec<RepoFile>
}

#[derive(Deserialize, Clone)]
struct RepoFile {
    path: String,
    hash: String,
    size: usize
}

impl RepoFile {
    fn get_fs_path(&self, root_dir: &Path) -> PathBuf {
        // Modern Windows versions support forward slashes anyways but it doesn't hurt to do something so trivial
        #[cfg(target_os = "windows")]
        return root_dir.join(&self.path.replace("/", "\\"));

        #[cfg(not(target_os = "windows"))]
        return root_dir.join(&self.path);
    }
}

#[derive(Clone)]
struct UpdateInfo {
    base_url: String,
    zip_url: String,
    zip_dir: String,
    files: Vec<RepoFile>, // only contains files needed for update
    is_new_repo: bool,
    cached_files: FnvHashMap<String, String>, // from repo cache
    size: usize,
    // New fields for better user communication
    update_size: usize,      // Size of changed files only
    total_size: usize,       // Total size of all files (for ZIP downloads)
    will_use_zip: bool,      // Whether ZIP download will be used
}

#[derive(Default, Clone)]
pub struct UpdateProgress {
    pub current: usize,
    pub total: usize
}

impl UpdateProgress {
    pub fn new(current: usize, total: usize) -> UpdateProgress {
        UpdateProgress {
            current,
            total
        }
    }
}

const REPO_CACHE_FILENAME: &str = ".tl_repo_cache";
#[derive(Serialize, Deserialize, Default)]
struct RepoCache {
    base_url: String,
    files: FnvHashMap<String, String> // path: hash
}

#[derive(Default)]
pub struct Updater {
    update_check_mutex: Mutex<()>,
    new_update: ArcSwap<Option<UpdateInfo>>,
    progress: ArcSwap<Option<UpdateProgress>>
}

const LOCALIZED_DATA_DIR: &str = "localized_data";
const CHUNK_SIZE: usize = 8192; // 8KiB
static NUM_THREADS: Lazy<usize> = Lazy::new(|| {
    let parallelism = thread::available_parallelism().unwrap().get();
    max(1, parallelism / 2)
});

// Hybrid update limits based on hosting platform
const INCREMENTAL_UPDATE_LIMIT_GITHUB: usize = 50;  // Conservative for GitHub
const INCREMENTAL_UPDATE_LIMIT_CDN: usize = 200;    // Aggressive for CDN/custom servers
const INCREMENTAL_SIZE_THRESHOLD: usize = 50 * 1024 * 1024; // 50MB

// Warn user if ZIP download is N times larger than actual changes
const ZIP_SIZE_WARNING_RATIO: f64 = 2.0;  // Warn if ZIP is 2x+ larger than changes

const MIN_CHUNK_SIZE: u64 = 1024 * 1024 * 5;

struct DownloadJob {
    agent: ureq::Agent,
    hasher: blake3::Hasher,
    buffer: [u8; CHUNK_SIZE]
}

impl DownloadJob {
    fn new() -> DownloadJob {
        DownloadJob {
            agent: ureq::Agent::new(),
            hasher: blake3::Hasher::new(),
            buffer: [0u8; CHUNK_SIZE]
        }
    }
}

impl Updater {
    pub fn check_for_updates(self: Arc<Self>, pedantic: bool) {
        std::thread::spawn(move || {
            if let Err(e) = self.check_for_updates_internal(pedantic) {
                error!("{}", e);
            }
        });
    }

    // Determine if a URL is hosted on GitHub
    fn is_github_hosted(url: &str) -> bool {
        url.contains("github.com") || 
        url.contains("githubusercontent.com") ||
        url.contains("github.io")
    }

    // Determine whether to use ZIP download based on multiple factors
    fn should_use_zip_download(
        file_count: usize,
        update_size: usize,
        base_url: &str
    ) -> bool {
        // Factor 1: Hosting platform determines file count limit
        let file_limit = if Self::is_github_hosted(base_url) {
            debug!("GitHub hosting detected, using conservative limit: {}", INCREMENTAL_UPDATE_LIMIT_GITHUB);
            INCREMENTAL_UPDATE_LIMIT_GITHUB
        } else {
            debug!("CDN/Custom hosting detected, using aggressive limit: {}", INCREMENTAL_UPDATE_LIMIT_CDN);
            INCREMENTAL_UPDATE_LIMIT_CDN
        };

        // Factor 2: Total size consideration
        let size_exceeds_threshold = update_size > INCREMENTAL_SIZE_THRESHOLD;
        
        if size_exceeds_threshold {
            debug!("Update size ({} MB) exceeds threshold, preferring ZIP download", 
                   update_size / (1024 * 1024));
        }

        // Decision: Use ZIP if either condition is met
        let use_zip = file_count > file_limit || size_exceeds_threshold;
        
        debug!(
            "Download strategy decision: files={}, limit={}, size={} MB, use_zip={}", 
            file_count, 
            file_limit, 
            update_size / (1024 * 1024),
            use_zip
        );

        use_zip
    }

    fn check_for_updates_internal(&self, pedantic: bool) -> Result<(), Error> {
        // Prevent multiple update checks running at the same time
        let Ok(_guard) = self.update_check_mutex.try_lock() else {
            return Ok(());
        };

        let hachimi = Hachimi::instance();
        let config = hachimi.config.load();
        let Some(index_url) = &config.translation_repo_index else {
            return Ok(());
        };
        let ld_dir_path = config.localized_data_dir.as_ref().map(|p| hachimi.get_data_path(p));

        if let Some(mutex) = Gui::instance() {
            mutex.lock().unwrap().show_notification(&t!("notification.checking_for_tl_updates"));
        }

        let index: RepoIndex = http::get_json(index_url)?;

        let cache_path = hachimi.get_data_path(REPO_CACHE_FILENAME);
        let repo_cache = if fs::metadata(&cache_path).is_ok() {
            let json = fs::read_to_string(&cache_path)?;
            serde_json::from_str(&json)?
        }
        else {
            RepoCache::default()
        };

        let is_new_repo = index.base_url != repo_cache.base_url;
        let mut update_files: Vec<RepoFile> = Vec::new();
        let mut update_size: usize = 0;
        let mut total_size: usize = 0;
        for file in index.files.iter() {
            if file.path.contains("..") || Path::new(&file.path).has_root() {
                warn!("File path '{}' sanitized", file.path);
                continue;
            }

            let updated = if is_new_repo {
                // redownload every single file because the directory will be deleted
                true
            }
            else if let Some(hash) = repo_cache.files.get(&file.path) {
                if hash == &file.hash {
                    // download if the file doesn't actually exist on disk (in pedantic mode)
                    pedantic && ld_dir_path.as_ref().map(|p| !p.join(&file.path).is_file()).unwrap_or(true)
                }
                else {
                    true
                }
            }
            else {
                // file doesnt exist yet, download it
                true
            };

            if updated {
                update_files.push(file.clone());
                update_size += file.size;
            }
            total_size += file.size;
        }

        if !update_files.is_empty() {
            // Determine download strategy
            let will_use_zip = Self::should_use_zip_download(
                update_files.len(),
                update_size,
                &index.base_url
            );

            // Calculate actual download size
            let actual_download_size = if will_use_zip { total_size } else { update_size };

            // Store update info with all relevant sizes
            self.new_update.store(Arc::new(Some(UpdateInfo {
                is_new_repo,
                base_url: index.base_url,
                zip_url: index.zip_url,
                zip_dir: index.zip_dir,
                files: update_files,
                cached_files: repo_cache.files,
                size: actual_download_size,
                update_size,
                total_size,
                will_use_zip,
            })));

            if let Some(mutex) = Gui::instance() {
                // Determine the dialog message based on download strategy
                let dialog_message = if will_use_zip && update_size > 0 {
                    let size_ratio = total_size as f64 / update_size as f64;
                    
                    if size_ratio >= ZIP_SIZE_WARNING_RATIO {
                        // Warn user about larger ZIP download
                        debug!(
                            "ZIP download warning: changed={} MB, total={} MB, ratio={:.2}x",
                            update_size / (1024 * 1024),
                            total_size / (1024 * 1024),
                            size_ratio
                        );
                        
                        t!(
                            "tl_update_dialog.content_zip_warning",
                            changed_size = Size::from_bytes(update_size),
                            download_size = Size::from_bytes(total_size)
                        )
                    } else {
                        // ZIP is being used but size difference is not significant
                        t!("tl_update_dialog.content", size = Size::from_bytes(actual_download_size))
                    }
                } else {
                    // Incremental update or no warning needed
                    t!("tl_update_dialog.content", size = Size::from_bytes(actual_download_size))
                };

                mutex.lock().unwrap().show_window(Box::new(SimpleYesNoDialog::new(
                    &t!("tl_update_dialog.title"),
                    &dialog_message,
                    |ok| {
                        if !ok { return; }
                        Hachimi::instance().tl_updater.clone().run();
                    }
                )));
            }
        }
        else if let Some(mutex) = Gui::instance() {
            mutex.lock().unwrap().show_notification(&t!("notification.no_tl_updates"));
        }
        
        Ok(())
    }

    pub fn run(self: Arc<Self>) {
        std::thread::spawn(move || {
            if let Err(e) = self.clone().run_internal() {
                error!("{}", e);
                self.progress.store(Arc::new(None));
                if let Some(mutex) = Gui::instance() {
                    mutex.lock().unwrap().show_notification(&t!("notification.update_failed", reason = e.to_string()));
                }
            }
        });
    }

    fn run_internal(self: Arc<Self>) -> Result<(), Error> {
        let Some(update_info) = (**self.new_update.load()).clone() else {
            return Ok(());
        };
        self.new_update.store(Arc::new(None));

        self.progress.store(Arc::new(Some(UpdateProgress::new(0, update_info.size))));
        if let Some(mutex) = Gui::instance() {
            mutex.lock().unwrap().update_progress_visible = true;
        }

        // Empty the localized data so files couldnt be accessed while update is in progress
        let hachimi = Hachimi::instance();
        hachimi.localized_data.store(Arc::new(LocalizedData::default()));

        // Clear the localized data if downloading from a new repo
        let localized_data_dir = hachimi.get_data_path(LOCALIZED_DATA_DIR);
        if update_info.is_new_repo {
            // rm -rf
            if let Ok(meta) = fs::metadata(&localized_data_dir) {
                if meta.is_dir() {
                    fs::remove_dir_all(&localized_data_dir)?;
                }
            }
        }

        fs::create_dir_all(&localized_data_dir)?;

        // Download the files - use the pre-determined strategy
        let cached_files = Arc::new(Mutex::new(update_info.cached_files.clone()));
        let error_count = if update_info.will_use_zip {
            self.clone().download_zip(&update_info, &localized_data_dir, cached_files.clone())
        }
        else {
            self.clone().download_incremental(&update_info, &localized_data_dir, cached_files.clone())
        }?;
        
        // Modify the config if needed
        if hachimi.config.load().localized_data_dir.is_none() {
            let mut config = (**hachimi.config.load()).clone();
            config.localized_data_dir = Some(LOCALIZED_DATA_DIR.to_owned());
            hachimi.save_and_reload_config(config)?;
        }

        // Drop the download state
        self.progress.store(Arc::new(None));

        // Reload the localized data
        hachimi.load_localized_data();

        // Save the repo cache (done last so if any of the previous fails, the entire update would be voided)
        let repo_cache = RepoCache {
            base_url: update_info.base_url.clone(),
            files: cached_files.lock().unwrap().clone()
        };
        let cache_path = hachimi.get_data_path(REPO_CACHE_FILENAME);
        utils::write_json_file(&repo_cache, &cache_path)?;

        if let Some(mutex) = Gui::instance() {
            let mut gui = mutex.lock().unwrap();
            gui.show_notification(&t!("notification.update_completed"));
            if error_count > 0 {
                gui.show_notification(&t!("notification.errors_during_update", count = error_count));
            }
        }
        Ok(())
    }

    fn download_incremental(
        self: Arc<Self>,
        update_info: &UpdateInfo,
        localized_data_dir: &Path,
        cached_files: Arc<Mutex<FnvHashMap<String, String>>>
    ) -> Result<usize, Error> {
        let total_size = update_info.size;
        let current_bytes = Arc::new(AtomicUsize::new(0));
        let non_fatal_error_count = Arc::new(AtomicUsize::new(0));
        let fatal_error = Arc::new(Mutex::new(None::<Error>));
        let stop_signal = Arc::new(AtomicBool::new(false));

        let (sender, receiver) = mpsc::channel::<RepoFile>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut handles = Vec::with_capacity(*NUM_THREADS);
        for _ in 0..*NUM_THREADS {
            let updater = self.clone();
            let localized_data_dir_clone = localized_data_dir.to_path_buf();
            let base_url_clone = update_info.base_url.clone();
            let cached_files_clone = Arc::clone(&cached_files);
            let current_bytes_clone = Arc::clone(&current_bytes);
            let non_fatal_error_count_clone = Arc::clone(&non_fatal_error_count);
            let fatal_error_clone = Arc::clone(&fatal_error);
            let stop_signal_clone = Arc::clone(&stop_signal);
            let receiver_clone = Arc::clone(&receiver);

            let handle = thread::Builder::new()
                .name("incremental_downloader".into())
                .spawn_with_priority(ThreadPriority::Min, move |result| {
                    if result.is_err() {
                        warn!("Failed to set background thread priority for incremental downloader.");
                    }
                    let mut job = DownloadJob::new();

                    while let Ok(repo_file) = receiver_clone.lock().unwrap().recv() {
                        if stop_signal_clone.load(atomic::Ordering::Relaxed) { break; }
                        
                        let file_path = repo_file.get_fs_path(&localized_data_dir_clone);
                        let url = utils::concat_unix_path(&base_url_clone, &repo_file.path);

                        let execute_result = (|| -> Result<String, Error> {
                            if let Some(parent) = Path::new(&file_path).parent() {
                                fs::create_dir_all(parent)?;
                            }
                            let mut file = fs::File::create(&file_path)?;
                            let res = job.agent.get(&url).call()?;
                            
                            http::download_file_buffered(res, &mut file, &mut job.buffer, |bytes| {
                                job.hasher.update(bytes);
                                let prev_size = current_bytes_clone.fetch_add(bytes.len(), atomic::Ordering::SeqCst);
                                updater.progress.store(Arc::new(Some(UpdateProgress::new(prev_size + bytes.len(), total_size))));
                            })?;

                            let hash = job.hasher.finalize().to_hex().to_string();
                            if hash != repo_file.hash {
                                return Err(Error::FileHashMismatch(file_path.to_str().unwrap_or("").to_string()));
                            }
                            job.hasher.reset();
                            Ok(hash)
                        })();

                        match execute_result {
                            Ok(hash) => {
                                cached_files_clone.lock().unwrap().insert(repo_file.path.clone(), hash);
                            },
                            Err(e) => {
                                if matches!(e, Error::OutOfDiskSpace | Error::FileHashMismatch(_)) {
                                    error!("Fatal error during incremental download: {}", e);
                                    *fatal_error_clone.lock().unwrap() = Some(e);
                                    stop_signal_clone.store(true, atomic::Ordering::Relaxed);
                                    return;
                                } else {
                                    error!("Non-fatal error during incremental download: {}", e);
                                    non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::SeqCst);
                                }
                            }
                        }
                    }
                }).unwrap();
            handles.push(handle);
        }

        for repo_file in update_info.files.iter() {
            if sender.send(repo_file.clone()).is_err() { break; }
        }
        drop(sender);

        for handle in handles {
            handle.join().unwrap();
        }

        if let Some(err) = fatal_error.lock().unwrap().take() {
            return Err(err);
        }

        Ok(non_fatal_error_count.load(atomic::Ordering::Relaxed))
    }

    fn download_zip(
        self: Arc<Self>,
        update_info: &UpdateInfo,
        localized_data_dir: &Path,
        cached_files: Arc<Mutex<FnvHashMap<String, String>>>
    ) -> Result<usize, Error> {
        let zip_path = localized_data_dir.join(".tmp.zip");
        let mut error_count = 0;

        {
            let total_size_header = ureq::agent().head(&update_info.zip_url).call()
                .ok()
                .and_then(|res| res.header("Content-Length").and_then(|s| s.parse::<usize>().ok()));

            let progress_total = match total_size_header {
                Some(size) if size > 0 => {
                    debug!("Using Content-Length from header for progress bar: {}", size);
                    size
                },
                _ => {
                    debug!("Server did not provide a valid Content-Length. Using fallback size from index: {}", update_info.size);
                    update_info.size
                }
            };

            let downloaded = Arc::new(AtomicUsize::new(0));
            let self_clone = self.clone();
            let downloaded_clone = downloaded.clone();

            let progress_bar = Arc::new(move |bytes_read: usize| {
                let prev_size = downloaded_clone.fetch_add(bytes_read, atomic::Ordering::Relaxed);
                let current = prev_size + bytes_read;
                self_clone.progress.store(Arc::new(Some(UpdateProgress::new(current, progress_total))));
            });

            http::download_file_parallel(
                &update_info.zip_url,
                &zip_path,
                *NUM_THREADS,
                MIN_CHUNK_SIZE,
                CHUNK_SIZE,
                progress_bar
            )?;

            let files_to_extract = Arc::new(
                update_info.files.iter()
                    .map(|f| (utils::concat_unix_path(&update_info.zip_dir, &f.path), f.clone()))
                    .collect::<FnvHashMap<_, _>>()
            );

            let zip_file = fs::File::open(&zip_path)?;
            let zip_archive = Arc::new(Mutex::new(zip::ZipArchive::new(zip_file)?));
            let total_size = update_info.size;
            let current_bytes = Arc::new(AtomicUsize::new(0));
            let non_fatal_error_count = Arc::new(AtomicUsize::new(0));
            let fatal_error = Arc::new(Mutex::new(None::<Error>));
            let stop_signal = Arc::new(AtomicBool::new(false));

            let (sender, receiver) = mpsc::channel::<usize>();
            let receiver = Arc::new(Mutex::new(receiver));
            let mut handles = Vec::with_capacity(*NUM_THREADS);
            for _ in 0..*NUM_THREADS {
                let updater = self.clone();
                let zip_archive_clone = Arc::clone(&zip_archive);
                let files_to_extract_clone = Arc::clone(&files_to_extract);
                let localized_data_dir_clone = localized_data_dir.to_path_buf();
                let cached_files_clone = Arc::clone(&cached_files);
                let current_bytes_clone = Arc::clone(&current_bytes);
                let non_fatal_error_count_clone = Arc::clone(&non_fatal_error_count);
                let fatal_error_clone = Arc::clone(&fatal_error);
                let stop_signal_clone = Arc::clone(&stop_signal);
                let receiver_clone = Arc::clone(&receiver);

                let handle = thread::Builder::new()
                    .name("zip_extractor".into())
                    .spawn_with_priority(ThreadPriority::Min, move |result| {
                        if result.is_err() {
                            warn!("Failed to set background thread priority for zip extractor.");
                        }

                        let mut buffer = vec![0u8; CHUNK_SIZE];
                        let mut hasher = blake3::Hasher::new();

                        while let Ok(i) = receiver_clone.lock().unwrap().recv() {
                            if stop_signal_clone.load(atomic::Ordering::Relaxed) { break; }
                            let mut archive_guard = zip_archive_clone.lock().unwrap();
                            
                            let mut zip_entry = match archive_guard.by_index(i) {
                                Ok(entry) => entry,
                                Err(_) => {
                                    non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::SeqCst);
                                    continue;
                                }
                            };
                            
                            let repo_file = match files_to_extract_clone.get(zip_entry.name()) {
                                Some(file) => file.clone(),
                                None => continue,
                            };
                        
                            let path = repo_file.get_fs_path(&localized_data_dir_clone);
                            if let Some(parent) = path.parent() {
                                if fs::create_dir_all(parent).is_err() {
                                    non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::SeqCst);
                                    continue;
                                }
                            }
                        
                            let mut out_file = match fs::File::create(&path) {
                                Ok(file) => file,
                                Err(_) => {
                                    non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::SeqCst);
                                    continue;
                                }
                            };

                            loop {
                                match zip_entry.read(&mut buffer) {
                                    Ok(0) => break,
                                    Ok(read_bytes) => {
                                        let data_slice = &buffer[..read_bytes];
                                        if out_file.write_all(data_slice).is_err() {
                                            *fatal_error_clone.lock().unwrap() = Some(Error::OutOfDiskSpace);
                                            stop_signal_clone.store(true, atomic::Ordering::Relaxed);
                                            return;
                                        }
                                        hasher.update(data_slice);
                                        let prev_size = current_bytes_clone.fetch_add(read_bytes, atomic::Ordering::SeqCst);
                                        updater.progress.store(Arc::new(Some(UpdateProgress::new(prev_size + read_bytes, total_size))));
                                    }
                                    Err(_) => {
                                        non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::SeqCst);
                                        break;
                                    }
                                }
                            }
                        
                            let hash = hasher.finalize().to_hex().to_string();
                            if hash != repo_file.hash {
                                let path_str = path.to_str().unwrap_or("").to_string();
                                *fatal_error_clone.lock().unwrap() = Some(Error::FileHashMismatch(path_str));
                                stop_signal_clone.store(true, atomic::Ordering::Relaxed);
                                return;
                            }
                            
                            cached_files_clone.lock().unwrap().insert(repo_file.path.clone(), hash);
                            hasher.reset();
                        }
                    }).unwrap();
                handles.push(handle);
            }

            let zip_len = zip_archive.lock().unwrap().len();
            for i in 0..zip_len {
                if sender.send(i).is_err() { break; }
            }
            drop(sender);

            for handle in handles {
                handle.join().unwrap();
            }
            
            if let Some(err) = fatal_error.lock().unwrap().take() { return Err(err); }
            error_count = non_fatal_error_count.load(atomic::Ordering::Relaxed);
        }

        if let Err(e) = fs::remove_file(&zip_path) {
            error!("Failed to remove temporary file '{}': {}", zip_path.display(), e);
            error_count += 1;
        }

        Ok(error_count)
    }

    pub fn progress(&self) -> Option<UpdateProgress> {
        (**self.progress.load()).clone()
    }
}
