use anyhow::{Error, Result};
use std::{
    ffi::OsStr,
    fs::canonicalize,
    path::Path,
    path::PathBuf,
    process::Command,
    str::from_utf8,
};
use solang::{
    cli::{
        Cli, Commands,
    },
    compile,
    doc,
    idl,
    languageserver,
    shell_complete,
};

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename)
        .extension()
        .and_then(OsStr::to_str)
}

pub fn build_solidity_contract(solidity_filename: String, build_release: &bool) -> Result<PathBuf, Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doc(doc_args) => doc(doc_args),
        Commands::Compile(compile_args) => compile(&compile_args),
        Commands::ShellComplete(shell_args) => shell_complete(Cli::command(), shell_args),
        Commands::LanguageServer(server_args) => languageserver::start_server(&server_args),
        Commands::Idl(idl_args) => idl::idl(&idl_args),
    }

    // let solidity_file_relative_path = format!("./{solidity_filename}");
    // let solidity_file_dir = PathBuf::from(solidity_file_relative_path);
    // let canonical_solidity_file_dir = canonicalize(&solidity_file_dir)?;
    // let exists_solidity_file = std::path::Path::new(&canonical_solidity_file_dir).exists();

    // let project_root_relative_path = format!("./");
    // let project_root_dir = PathBuf::from(project_root_relative_path);
    // let canonical_project_root_dir = canonicalize(&project_root_dir)?;

    // let compilers_shell_script_relative_path = format!("./compilers.sh");
    // let compilers_shell_script_file_dir = PathBuf::from(compilers_shell_script_relative_path);
    // let canonical_compilers_shell_script_file_dir = canonicalize(&compilers_shell_script_file_dir)?;

    // if get_extension_from_filename(&solidity_filename) == Some("sol") && exists_solidity_file {
    //     println!("Found file {:?} with Solidity file extension in the project root", solidity_filename);

    //     let output = if cfg!(target_os = "windows") {
    //         println!("Detected Windows OS");
    //         Command::new("cmd")
    //             // project root directory
    //             .current_dir(canonical_project_root_dir.clone())
    //             .arg(format!("{:?} {:?} {:?} {:?}",
    //                 canonical_compilers_shell_script_file_dir.display(), &solidity_filename, &canonical_solidity_file_dir, &build_release))
    //             .output()
    //             .expect("failed to execute process")
    //     } else {
    //         Command::new("sh")
    //             // project root directory
    //             .current_dir(canonical_project_root_dir.clone())
    //             .arg("-c")
    //             .arg(format!("{:?} {:?} {:?} {:?}",
    //                 canonical_compilers_shell_script_file_dir.display(), &solidity_filename, &canonical_solidity_file_dir, &build_release))
    //             .output()
    //             .expect("failed to execute process")
    //     };
    //     let output = output.stdout;
    //     println!("output: {:#?}", from_utf8(&output).unwrap());
    //     return Ok(canonical_project_root_dir)
    // } else {
    //     anyhow::bail!("Unable to find a filename {:?} with .sol file extension in the project root folder", solidity_filename);
    // }
}
