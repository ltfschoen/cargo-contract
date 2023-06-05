use anyhow::{Error, Result};
use std::{
    fs::canonicalize,
    path::PathBuf,
    process::Command,
    str::from_utf8,
};

// compile Solidity smart contract to WASM using Solang `solang`
pub fn build_solidity_contract(
    emit: &str,
    contract: &str,
    no_constant_folding: &str,
    no_strength_reduce: &str,
    optimizer_level: &str,
    no_dead_storage: &str,
    target: &str,
    address_length: &str,
    no_vector_to_slice: &str,
    no_cse: &str,
    value_length: &str,
    standard_json: &str,
    verbose: &str,
    output_dir: &str,
    output_meta: &str,
    import_path: &str,
    import_map: &str,
    no_log_api_return_codes: &str,
    no_log_runtime_errors: &str,
    no_print: &str,
    release: &str,
    solidity_filename: &str,
) -> Result<(), Error> {
    // TODO - refactor so not repeating code in build.rs
    let project_root_relative_path = format!("./");
    let project_root_dir = PathBuf::from(project_root_relative_path);
    let canonical_project_root_dir = canonicalize(&project_root_dir)?;
    println!("canonical_project_root_dir: {}", canonical_project_root_dir.display());
    let os_string = canonical_project_root_dir.clone().into_os_string();
    let canonical_project_root_dir_str = os_string.clone().into_string().unwrap();
    println!("canonical_project_root_dir_str: {}", canonical_project_root_dir_str);

    let empty = "".to_string();
    let mut used_output_dir: String = "".to_string();
    if output_meta == empty {
        if output_dir == empty {
            used_output_dir = canonical_project_root_dir_str;
        } else {
            used_output_dir = output_dir.to_string();
        }
    } else {
        used_output_dir = output_meta.to_string();
    }

    // Detect if `solang` binary exists in PATH
    match Command::new("solang").spawn() {
        Ok(_) => {
            println!("Detected solang binary...\n");
            // to get here the user ran `cargo contract build ...`
            println!("Ready to build using Solang Compiler for Substrate.\n");
            println!("Ready to generating ABI .contract and contract .wasm files in {:?}.\n", used_output_dir.to_string());
        },
        Err(e) => {
            if let std::io::ErrorKind::NotFound = e.kind() {
                println!("`solang` command could not be found.\n\n");
                println!("Please follow the installation instructions at https://github.com/hyperledger/solang then check your PATH and try again...\n\n");
            } else {
                println!("Error encountered trying to detect `solang` {:#?}", e);
            }
        },
    }

    // if you run: `cargo run -p cargo-contract contract build -v --release --help` then that should translate to running:
    // e.g. `solang compile --target substrate -v --release --help`
    //
    // or to compile run: `cargo run -p cargo-contract contract build --contract flipper -v --release --solidity-filename /Users/.../cargo-contract/flipper.sol`

    let arr = vec![
        "solang".to_string(),
        "compile".to_string(),
        emit.to_string(),
        contract.to_string(),
        no_constant_folding.to_string(),
        no_strength_reduce.to_string(),
        optimizer_level.to_string(),
        no_dead_storage.to_string(),
        target.to_string(),
        address_length.to_string(),
        no_vector_to_slice.to_string(),
        no_cse.to_string(),
        value_length.to_string(),
        standard_json.to_string(),
        verbose.to_string(),
        output_dir.to_string(),
        output_meta.to_string(),
        import_path.to_string(),
        import_map.to_string(),
        no_log_api_return_codes.to_string(),
        no_log_runtime_errors.to_string(),
        no_print.to_string(),
        release.to_string(),
        solidity_filename.to_string(),
    ];
    let arg: String = arr.join(" ");
    println!("{}", arg);

    // note: unable to get it to detect subcommand `compile` if use `Command::new("solang")` without `.arg("-c")`
    // hence why using `sh` instead.
    let output = if cfg!(target_os = "windows") {
        // https://doc.rust-lang.org/std/process/struct.Command.html#
        Command::new("cmd")
            .current_dir(canonical_project_root_dir.clone())
            .arg("/C")
            .arg(arg)
            .output()
            .expect("failed to execute process")
    } else {
        Command::new("sh")
            .current_dir(canonical_project_root_dir.clone())
            .arg("-c")
            .arg(arg)
            .output()
            .expect("failed to execute process")
    };
    let output = output.stdout;
    println!("output: {}", from_utf8(&output).unwrap());

    return Ok(())
}
