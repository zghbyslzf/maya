pub mod error;
pub mod file_utils;

pub use error::{Error, Result};
pub use file_utils::{
    create_zip_archive, find_file, find_files, find_files_by_extension, find_by_name, MatchType,
    remove_empty_dirs,
}; 