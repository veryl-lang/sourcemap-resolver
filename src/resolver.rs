use sourcemap::SourceMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const LINK_HEADER: &str = "//# sourceMappingURL=";

#[derive(Debug)]
pub struct ResolveResult {
    /// Path to source file
    pub path: PathBuf,
    /// 1-indexed line number
    pub line: u32,
    /// 1-indexed column number
    pub column: u32,
}

#[derive(Error, Debug)]
pub enum ResolveError {
    #[error("I/O error")]
    IO(#[from] std::io::Error),

    #[error("SourceMap error")]
    SourceMap(#[from] sourcemap::Error),

    #[error("Mapping not found")]
    MappingNotFound,

    #[error("Token not found")]
    TokenNotFound,

    #[error("Path not found")]
    PathNotFound,
}

pub fn resolve<T: AsRef<Path>>(
    path: T,
    line: u32,
    column: Option<u32>,
) -> Result<ResolveResult, ResolveError> {
    let src = fs::read_to_string(path.as_ref())?;

    let Some(last_line) = src.lines().last() else {
        return Err(ResolveError::MappingNotFound);
    };

    if !last_line.starts_with(LINK_HEADER) {
        return Err(ResolveError::MappingNotFound);
    }

    let map_path = last_line.strip_prefix(LINK_HEADER).unwrap();
    let map_path = path.as_ref().parent().unwrap().join(map_path);

    let map_text = fs::read(&map_path)?;
    let source_map = SourceMap::from_reader(map_text.as_slice())?;

    // sourcemap crate is based on 0-indexed position
    let line = line - 1;
    let column = column.unwrap_or(1) - 1;

    let token = source_map
        .lookup_token(line, column)
        .ok_or(ResolveError::TokenNotFound)?;

    let path = token.get_source().ok_or(ResolveError::PathNotFound)?;
    let path = map_path.parent().unwrap().join(path);
    let path = fs::canonicalize(path)?;
    let line = token.get_src_line() + 1;
    let column = token.get_src_col() + 1;

    Ok(ResolveResult { path, line, column })
}
