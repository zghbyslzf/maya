pub mod error;
pub mod file_utils;

pub use error::{Error, Result};
pub use file_utils::{
    atomic_replace_directory, atomic_write, create_zip_archive, find_by_name,
    find_directories_by_name_pruned, find_file, find_files, find_files_by_extension,
    remove_empty_dirs, MatchType,
};
