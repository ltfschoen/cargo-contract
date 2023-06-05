// Copyright 2018-2022 Parity Technologies (UK) Ltd.
// This file is part of cargo-contract.
//
// cargo-contract is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// cargo-contract is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with cargo-contract.  If not, see <http://www.gnu.org/licenses/>.

use anyhow::Result;
use contract_build::{
    BuildArtifacts,
    BuildMode,
    BuildResult,
    ExecuteArgs,
    Features,
    ManifestPath,
    Network,
    OptimizationPasses,
    OutputType,
    Target,
    UnstableFlags,
    UnstableOptions,
    Verbosity,
    VerbosityFlags,
};
use std::{
    convert::TryFrom,
    ffi::OsStr,
    fs::canonicalize,
    path::Path,
    path::PathBuf,
};
use crate::{
    cmd::solidity::build_solidity_contract,
};

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename)
        .extension()
        .and_then(OsStr::to_str)
}

/// Executes build of the smart contract which produces a Wasm binary that is ready for
/// deploying.
///
/// It does so by invoking `cargo build` and then post processing the final binary.
#[derive(Debug, clap::Args)]
#[clap(name = "build")]
pub struct BuildCommand {
    /// Path to the `Cargo.toml` of the contract to build
    #[clap(long, value_parser)]
    manifest_path: Option<PathBuf>,
    /// By default the contract is compiled with debug functionality
    /// included. This enables the contract to output debug messages,
    /// but increases the contract size and the amount of gas used.
    ///
    /// A production contract should always be build in `release` mode!
    /// Then no debug functionality is compiled into the contract.
    #[clap(long = "release")]
    build_release: bool,
    /// Build offline
    #[clap(long = "offline")]
    build_offline: bool,
    /// Performs linting checks during the build process
    #[clap(long)]
    lint: bool,
    /// Which build artifacts to generate.
    ///
    /// - `all`: Generate the Wasm, the metadata and a bundled `<name>.contract` file.
    ///
    /// - `code-only`: Only the Wasm is created, generation of metadata and a bundled
    ///   `<name>.contract` file is skipped.
    ///
    /// - `check-only`: No artifacts produced: runs the `cargo check` command for the
    ///   Wasm target, only checks for compilation errors.
    #[clap(long = "generate", value_enum, default_value = "all")]
    build_artifact: BuildArtifacts,
    #[clap(flatten)]
    features: Features,
    #[clap(flatten)]
    verbosity: VerbosityFlags,
    #[clap(flatten)]
    unstable_options: UnstableOptions,
    /// Number of optimization passes, passed as an argument to `wasm-opt`.
    ///
    /// - `0`: execute no optimization passes
    ///
    /// - `1`: execute 1 optimization pass (quick & useful opts, useful for iteration
    ///   builds)
    ///
    /// - `2`, execute 2 optimization passes (most opts, generally gets most perf)
    ///
    /// - `3`, execute 3 optimization passes (spends potentially a lot of time
    ///   optimizing)
    ///
    /// - `4`, execute 4 optimization passes (also flatten the IR, which can take a lot
    ///   more time and memory but is useful on more nested / complex / less-optimized
    ///   input)
    ///
    /// - `s`, execute default optimization passes, focusing on code size
    ///
    /// - `z`, execute default optimization passes, super-focusing on code size
    ///
    /// - The default value is `z`
    ///
    /// - It is possible to define the number of optimization passes in the
    ///   `[package.metadata.contract]` of your `Cargo.toml` as e.g. `optimization-passes
    ///   = "3"`. The CLI argument always takes precedence over the profile value.
    #[clap(long)]
    optimization_passes: Option<OptimizationPasses>,
    /// Do not remove symbols (Wasm name section) when optimizing.
    ///
    /// This is useful if one wants to analyze or debug the optimized binary.
    #[clap(long)]
    keep_debug_symbols: bool,
    /// Export the build output in JSON format.
    #[clap(long, conflicts_with = "verbose")]
    output_json: bool,
    /// Don't perform wasm validation checks e.g. for permitted imports.
    #[clap(long)]
    skip_wasm_validation: bool,
    /// Which bytecode to build the contract into.
    #[clap(long, default_value = "wasm")]
    target: Target,
    /// The maximum number of pages available for a wasm contract to allocate.
    #[clap(long, default_value_t = contract_build::DEFAULT_MAX_MEMORY_PAGES)]
    max_memory_pages: u32,
    /// Solang CLI
    ///
    /// Only build .sol Solidity files in the project root using Solang.
    #[clap(long)]
    solang: bool,
    #[clap(long)]
    emit: Option<String>,
    // filter so only compile these contract names
    #[clap(long)]
    contract: Option<String>,
    #[clap(long)]
    no_constant_folding: bool,
    #[clap(long)]
    no_strength_reduce: bool,
    #[clap(short('O'), long)]
    optimizer_level: Option<String>,
    #[clap(long)]
    no_dead_storage: bool,
    #[clap(long)]
    address_length: Option<u8>,
    #[clap(long)]
    no_vector_to_slice: bool,
    #[clap(long)]
    no_cse: bool,
    #[clap(long)]
    value_length: Option<u8>,
    #[clap(long)]
    standard_json: bool,
    #[clap(short('o'), long)]
    output: Option<String>,
    #[clap(long)]
    output_meta: Option<String>,
    #[clap(name = "importpath", short('I'), long)]
    import_path: Option<String>,
    #[clap(name = "importmap", short('m'), long)]
    import_map: Option<String>,
    #[clap(long)]
    no_log_api_return_codes: bool,
    #[clap(long)]
    no_log_runtime_errors: bool,
    #[clap(long)]
    no_print: bool,
    /// specified multiple times when using
    /// https://docs.rs/clap/latest/clap/builder/struct.Arg.html#method.num_args
    #[clap(long, num_args(1..), value_terminator(" "))]
    solidity_filename: Option<String>,
}

impl BuildCommand {
    pub fn exec(&self) -> Result<BuildResult> {
        if self.solang {
            println!("Processing Solang");

            // TODO - reuse most of this to generate `canonical_solidity_file_dir`
            let project_root_relative_path = format!("./");
            let project_root_dir = PathBuf::from(project_root_relative_path);
            let canonical_project_root_dir: PathBuf = canonicalize(&project_root_dir)?;
            let os_string = canonical_project_root_dir.clone().into_os_string();
            let canonical_project_root_dir_str = os_string.into_string().unwrap();

            let _solidity_filename = match &self.solidity_filename {
                Some(s) => s,
                None => anyhow::bail!("Unable to find solidity_filename: {:?}", &self.solidity_filename),
            };

            // TODO - since `solidity_filename` should be able to support multiple(true)
            // options `i.e. ... --solidity-file /path/to/x.sol --solidity-file /path/to/y.sol ...`
            // we should loop through them and pass multiple args.
            // This also applies to for `importpath` and `importmap`
            let solidity_file_relative_path = format!("{}", _solidity_filename);
            let solidity_file_dir = PathBuf::from(solidity_file_relative_path);
            println!("solidity_file_dir: {:?}", solidity_file_dir);
            let canonical_solidity_file_dir = canonicalize(&solidity_file_dir)?;
            println!("canonical_solidity_file_dir: {:?}", solidity_file_dir);
            let exists_solidity_file = std::path::Path::new(&canonical_solidity_file_dir).exists();
            println!("exists_solidity_file: {:?}", exists_solidity_file);

            if get_extension_from_filename(&_solidity_filename) != Some("sol") || !exists_solidity_file {
                anyhow::bail!("Unable to find file {:?} with Solidity file extension in the project root", &_solidity_filename);
            }

            println!("Found file {:?} with Solidity file extension in the project root", _solidity_filename);

            let empty: String = "".to_string();
            let mut _emit: String = empty.to_string();
            if let Some(emit) = &self.emit {
                let arr = vec!["--emit", " ", emit];
                _emit = arr.concat();
            }

            let mut _contract: String = empty.to_string();
            if let Some(contract) = &self.contract {
                let arr = vec!["--contract", " ", contract];
                _contract = arr.concat();
            }

            let mut _no_constant_folding: String = empty.to_string();
            if self.no_constant_folding == true {
                _no_constant_folding = "--no-constant-folding".to_string();
            }

            let mut _no_strength_reduce: String = empty.to_string();
            if self.no_strength_reduce == true {
                _no_strength_reduce = "--no-strength-reduce".to_string();
            }

            let mut _optimizer_level: String = empty.to_string();
            if let Some(optimizer_level) = &self.optimizer_level {
                let arr = vec!["-O", " ", optimizer_level];
                _optimizer_level = arr.concat();
            }

            let mut _no_dead_storage: String = empty.to_string();
            if self.no_dead_storage == true {
                _no_dead_storage = "--no-dead-storage".to_string();
            }

            // note: Solang option `--target` is hard-coded to value `"substrate"`
            // note: must specify a `--target` for it to compile
            let _target = "--target substrate".to_string();

            let mut _address_length: String = empty.to_string();
            if let Some(address_length) = &self.address_length {
                let bind = address_length.to_string();
                let arr = vec!["--address-length", " ", bind.as_str()];
                _address_length = arr.concat();
            }

            let mut _no_vector_to_slice: String = empty.to_string();
            if self.no_vector_to_slice == true {
                _no_vector_to_slice = "--no-vector-to-slice".to_string();
            }

            let mut _no_cse: String = empty.to_string();
            if self.no_cse == true {
                _no_cse = "--no-cse".to_string();
            }

            let mut _value_length: String = empty.to_string();
            if let Some(value_length) = &self.value_length {
                let bind = value_length.to_string();
                let arr = vec!["--value-length", " ", bind.as_str()];
                _value_length = arr.concat();
            }

            let mut _standard_json: String = empty.to_string();
            if self.standard_json == true {
                _standard_json = "--standard-json".to_string();
            }

            let mut _verbosity = TryFrom::<&VerbosityFlags>::try_from(&self.verbosity)?;
            let mut _verbose: String = empty.to_string();
            if _verbosity == Verbosity::Verbose {
                _verbose = "--verbose".to_string();
            }

            let mut _output_dir: String = empty.to_string();
            if let Some(output) = &self.output {
                let arr = vec!["--output", " ", output];
                _output_dir = arr.concat();
            }

            let mut _output_meta: String = empty.to_string();
            if let Some(output_meta) = &self.output_meta {
                let arr = vec!["--output-meta", " ", output_meta];
                _output_meta = arr.concat();
            }

            let mut _import_path: String = empty.to_string();
            if let Some(import_path) = &self.import_path {
                let arr = vec!["-I", " ", import_path];
                _import_path = arr.concat();
            }

            let mut _import_map: String = empty.to_string();
            if let Some(import_map) = &self.import_map {
                let arr = vec!["-m", " ", import_map];
                _import_map = arr.concat();
            }

            let mut _no_log_api_return_codes: String = empty.to_string();
            if self.no_log_api_return_codes == true {
                _no_log_api_return_codes = "--no-log-api-return-codes".to_string();
            }

            let mut _no_log_runtime_errors: String = empty.to_string();
            if self.no_log_runtime_errors == true {
                _no_log_runtime_errors = "--no-log-runtime-errors".to_string();
            }

            let mut _no_print: String = empty.to_string();
            if self.no_print == true {
                _no_print = "--no-print".to_string();
            }

            // `cargo-contract` option of `--release` causes `self.build_release` variable to be `"true"`
            // so translate to a value of `"--release"` to be used as a `solang` CLI option
            // note: use arg `self.build_release` for `--release`
            let mut _release: String = empty.to_string();
            if self.build_release == true {
                _release = "--release".to_string();
            }

            build_solidity_contract(
                &_emit,
                &_contract,
                &_no_constant_folding,
                &_no_strength_reduce,
                &_optimizer_level,
                &_no_dead_storage,
                &_target,
                &_address_length,
                &_no_vector_to_slice,
                &_no_cse,
                &_value_length,
                &_standard_json,
                // note: Solang option `--verbose` is going to use cargo-contract's existing `--verbose` option
                &_verbose,
                &_output_dir,
                &_output_meta,
                &_import_path,
                &_import_map,
                &_no_log_api_return_codes,
                &_no_log_runtime_errors,
                &_no_print,
                &_release,
                &_solidity_filename,
            )?;

            println!("_output_dir.clone().into(): {:?}", _output_dir.clone());
            println!("_output_meta.clone().into(): {:?}", _output_meta.clone());

            // return dummy data to indicate success
            // TODO - fix this to match CLI arguments provided for Solang
            return Ok(
                BuildResult {
                    // target_directory: canonical_project_root_dir.clone(),
                    target_directory: _output_dir.clone().into(),
                    build_mode: match self.build_release {
                        true => BuildMode::Release,
                        false => BuildMode::Debug,
                    },
                    build_artifact: BuildArtifacts::All,
                    verbosity: match _verbosity {
                        Verbosity::Verbose => Verbosity::Verbose,
                        _ => Verbosity::Default,
                    },
                    output_type: OutputType::Json,
                    // dest_wasm: Some(canonical_project_root_dir.clone()),
                    dest_wasm: Some(_output_meta.clone().into()),
                    metadata_result: None,
                    // note: multiple files could be compiled using Solang
                    optimization_result: None,
                }
            )
        }

        let manifest_path = ManifestPath::try_from(self.manifest_path.as_ref())?;

        let unstable_flags: UnstableFlags =
            TryFrom::<&UnstableOptions>::try_from(&self.unstable_options)?;
        let mut verbosity = TryFrom::<&VerbosityFlags>::try_from(&self.verbosity)?;

        let build_mode = match self.build_release {
            true => BuildMode::Release,
            false => BuildMode::Debug,
        };

        let network = match self.build_offline {
            true => Network::Offline,
            false => Network::Online,
        };

        let output_type = match self.output_json {
            true => OutputType::Json,
            false => OutputType::HumanReadable,
        };

        // We want to ensure that the only thing in `STDOUT` is our JSON formatted string.
        if matches!(output_type, OutputType::Json) {
            verbosity = Verbosity::Quiet;
        }

        let args = ExecuteArgs {
            manifest_path,
            verbosity,
            build_mode,
            features: self.features.clone(),
            network,
            build_artifact: self.build_artifact,
            unstable_flags,
            optimization_passes: self.optimization_passes,
            keep_debug_symbols: self.keep_debug_symbols,
            lint: self.lint,
            output_type,
            skip_wasm_validation: self.skip_wasm_validation,
            target: self.target,
            max_memory_pages: self.max_memory_pages,
        };

        contract_build::execute(args)
    }
}

#[derive(Debug, clap::Args)]
#[clap(name = "check")]
pub struct CheckCommand {
    /// Path to the `Cargo.toml` of the contract to build
    #[clap(long, value_parser)]
    manifest_path: Option<PathBuf>,
    #[clap(flatten)]
    verbosity: VerbosityFlags,
    #[clap(flatten)]
    features: Features,
    #[clap(flatten)]
    unstable_options: UnstableOptions,
    #[clap(long, default_value = "wasm")]
    target: Target,
}

impl CheckCommand {
    pub fn exec(&self) -> Result<BuildResult> {
        let manifest_path = ManifestPath::try_from(self.manifest_path.as_ref())?;
        let unstable_flags: UnstableFlags =
            TryFrom::<&UnstableOptions>::try_from(&self.unstable_options)?;
        let verbosity: Verbosity = TryFrom::<&VerbosityFlags>::try_from(&self.verbosity)?;

        let args = ExecuteArgs {
            manifest_path,
            verbosity,
            build_mode: BuildMode::Debug,
            features: self.features.clone(),
            network: Network::default(),
            build_artifact: BuildArtifacts::CheckOnly,
            unstable_flags,
            optimization_passes: Some(OptimizationPasses::Zero),
            keep_debug_symbols: false,
            lint: false,
            output_type: OutputType::default(),
            skip_wasm_validation: false,
            target: self.target,
            max_memory_pages: 0,
        };

        contract_build::execute(args)
    }
}
