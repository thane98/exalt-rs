use std::collections::HashSet;
use std::path::{Path, PathBuf};

use exalt_ast::surface::{Decl, IncludePathComponent, Script};
use exalt_ast::Location;
use normpath::PathExt;

use crate::reporting::ParserError;
use crate::{parser, CompilerLog};

type Result<T> = std::result::Result<T, ParserError>;

fn construct_fs_path(source_path: &[IncludePathComponent]) -> PathBuf {
    let mut buf = PathBuf::new();
    for component in source_path {
        match component {
            IncludePathComponent::Node(name) => buf.push(name),
            IncludePathComponent::Parent => buf.push(".."),
        }
    }
    buf
}

fn find_script(path: &[IncludePathComponent], search_paths: &[PathBuf]) -> Option<PathBuf> {
    let path = construct_fs_path(path);
    for search_path in search_paths {
        let mut full_path = search_path.join(&path);
        full_path.set_extension("exl");
        if full_path.exists() && full_path.is_file() {
            return Some(full_path);
        }
    }
    None
}

fn build_search_paths(location: Location, current_file_path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    // Parent dir of the current file
    paths.push(
        current_file_path
            .parent()
            .ok_or_else(|| ParserError::IncludeError(location.clone()))?
            .to_path_buf(),
    );
    // Path of the compiler exe
    // This is where we expect to find the standard library
    paths.push(
        std::env::current_exe()
            .map_err(|_| ParserError::IncludeError(location.clone()))?
            .parent()
            .ok_or_else(|| ParserError::IncludeError(location.clone()))?
            .normalize()
            .map_err(|_| {
                ParserError::PathNormalizationError(
                    location.clone(),
                    current_file_path.to_path_buf(),
                )
            })?
            .into_path_buf(),
    );
    Ok(paths)
}

fn pull_in_scripts_recursive(
    location: Location,
    path: PathBuf,
    script: Script,
    log: &mut CompilerLog,
    included_paths: &mut HashSet<PathBuf>,
    scripts: &mut Vec<Script>,
) -> Result<()> {
    let search_paths = build_search_paths(location, &path)?;
    included_paths.insert(path);
    for decl in &script.0 {
        if let Decl::Include { location, path } = decl {
            // Find the file in the source paths and load it.
            let source_path = find_script(path, &search_paths)
                .ok_or_else(|| ParserError::IncludeNotFound(location.clone()))?
                .normalize()
                .map_err(|_| ParserError::IncludeError(location.clone()))?
                .into_path_buf();
            // Only try to pull in the file if it hasn't been included yet.
            if !included_paths.contains(&source_path) {
                let contents = std::fs::read_to_string(&source_path)
                    .map_err(|_| ParserError::IncludeError(location.clone()))?;
                let script = parser::parse(log.peek_file_id(), &contents, log);
                log.add(source_path.to_string_lossy().to_string(), contents.clone());
                pull_in_scripts_recursive(
                    location.clone(),
                    source_path,
                    script,
                    log,
                    included_paths,
                    scripts,
                )?;
            }
        }
    }
    scripts.push(script);
    Ok(())
}

pub fn build_script_with_includes(
    path: PathBuf,
    script: Script,
    log: &mut CompilerLog,
) -> Result<Script> {
    let mut included_paths = HashSet::new();
    let mut scripts = Vec::new();
    let normalized_path = path
        .normalize()
        .map_err(|_| ParserError::PathNormalizationError(Location::Generated, path.clone()))?
        .into_path_buf();
    pull_in_scripts_recursive(
        Location::Generated,
        normalized_path,
        script,
        log,
        &mut included_paths,
        &mut scripts,
    )?;
    Ok(Script(scripts.into_iter().flat_map(|s| s.0).collect()))
}
