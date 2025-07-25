use std::{collections::HashMap, path::Path};

#[allow(clippy::struct_excessive_bools)]
pub struct CompilerOptions {
    pub optimization_strategy: OptimizationStrategy,
    pub enable_bulk_memory: bool,
    pub enable_sign_extension: bool,
    pub enable_nontrapping_f2i: bool,
    pub enable_export_memory: bool,
    pub flag_use: HashMap<String, String>,
    pub trap_on_abort: bool,
    pub runtime: RuntimeStrategy,
    pub source: String,
}

impl CompilerOptions {
    pub fn default_for(library_source: impl Into<String>) -> Self {
        Self {
            source: library_source.into(),
            // By default, trap on abort.
            // This makes that the module has no 'env' dependency to handle failure.
            trap_on_abort: true,
            // Other options are set to default
            enable_bulk_memory: false,
            enable_nontrapping_f2i: false,
            enable_export_memory: false,
            enable_sign_extension: false,
            flag_use: HashMap::default(),
            optimization_strategy: OptimizationStrategy::default(),
            runtime: RuntimeStrategy::default(),
        }
    }
}

#[derive(Default)]
pub enum OptimizationStrategy {
    O1,
    O2,
    #[default]
    O3,
}

#[derive(Default)]
pub enum RuntimeStrategy {
    #[default]
    Incremental,
    Minimal,
    Stub,
}

impl CompilerOptions {
    pub(crate) fn to_npx_command(&self, source_path: &Path, output_path: &Path) -> String {
        let flag_bulk_memory = if self.enable_bulk_memory {
            ""
        } else {
            "--disable bulk-memory "
        };

        let flag_sign_extension = if self.enable_sign_extension {
            ""
        } else {
            "--disable sign-extension "
        };

        let flag_non_trapping_f2i = if self.enable_nontrapping_f2i {
            ""
        } else {
            "--disable nontrapping-f2i "
        };

        let flag_export_memory = if self.enable_export_memory {
            ""
        } else {
            "--noExportMemory "
        };

        let flag_runtime = match self.runtime {
            RuntimeStrategy::Minimal => "--runtime minimal ",
            RuntimeStrategy::Incremental => "--runtime incremental ",
            RuntimeStrategy::Stub => "--runtime stub ",
        };

        let flag_optimization = match self.optimization_strategy {
            OptimizationStrategy::O1 => "-O1 ",
            OptimizationStrategy::O2 => "-O2 ",
            OptimizationStrategy::O3 => "-O3 ",
        };

        let flag_use = match (self.flag_use.is_empty(), self.trap_on_abort) {
            // No custom flags, no trap on abort
            (true, false) => String::new(),
            // Custom flags but no trap on abort
            (false, false) => format!(
                "--use {} ",
                self.flag_use
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            // Trap on abort
            (true, true) | (false, true) => {
                format!(
                    "--lib . --use {} ",
                    self.flag_use
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .chain(vec![("abort", "custom_abort")]) // include trap
                        .map(|(key, value)| format!("{key}={value}"))
                        .collect::<Vec<String>>()
                        .join(" ")
                )
            }
        };

        cfg_if::cfg_if! {
            if #[cfg(feature = "nodejs")] {
                fn runtime() -> &'static str { "node ./node_modules/assemblyscript/bin/asc.js" }
            } else if #[cfg(feature = "bun")] {
                fn runtime() -> &'static str { "~/.bun/bin/bunx assemblyscript@0.27.27/asc" }
            } else if #[cfg(feature = "deno")] {
                fn runtime() -> &'static str { "deno run --allow-read --allow-write --allow-env 'npm:assemblyscript@0.27.27/asc'" }
            } else {
                compile_error!("Invalid feature for AssemblyScript compiler runtime")
            }
        };

        format!(
            concat!(
                // Pass input file & output file to command
                "{compiler_runtime_command} {source_path:?} -o {output_path:?} ",
                // Pas additional options to command
                "{flag_optimization}",
                "{flag_bulk_memory}",
                "{flag_sign_extension}",
                "{flag_non_trapping_f2i}",
                "{flag_runtime}",
                "{flag_export_memory}",
                "{flag_use}",
            ),
            compiler_runtime_command = runtime(),
            source_path = &source_path,
            output_path = &output_path,
            flag_bulk_memory = flag_bulk_memory,
            flag_sign_extension = flag_sign_extension,
            flag_non_trapping_f2i = flag_non_trapping_f2i,
            flag_runtime = flag_runtime,
            flag_export_memory = flag_export_memory,
            flag_optimization = flag_optimization,
            flag_use = flag_use,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_creation() {
        let conf = CompilerOptions::default_for("".to_string());
        let source_path = PathBuf::from("source_path");
        let output_path = PathBuf::from("output_path");

        assert!(
            conf.to_npx_command(&source_path, &output_path)
                .contains(concat!(
                    "-o \"output_path\"",
                    " -O3 ",
                    "--disable bulk-memory ",
                    "--disable sign-extension ",
                    "--disable nontrapping-f2i ",
                    "--runtime incremental ",
                    "--noExportMemory ",
                    "--lib . --use abort=custom_abort ",
                ))
        );
    }

    #[test]
    fn test_to_npx() {
        let mut options = CompilerOptions {
            optimization_strategy: OptimizationStrategy::O1,
            enable_bulk_memory: true,
            enable_sign_extension: true,
            enable_nontrapping_f2i: true,
            enable_export_memory: true,
            flag_use: HashMap::new(),
            trap_on_abort: true,
            runtime: super::RuntimeStrategy::Incremental,
            source: "".to_string(),
        };

        let source_path = PathBuf::from("path").join("to").join("source");
        let output_path = PathBuf::from("path").join("to").join("output");

        assert!(
            options
                .to_npx_command(&source_path, &output_path)
                .contains(concat!(
                    "-o \"path/to/output\" ",
                    "-O1 --runtime incremental ",
                    "--lib . --use abort=custom_abort ",
                ))
        );

        options = CompilerOptions {
            optimization_strategy: OptimizationStrategy::O2,
            enable_bulk_memory: false,
            enable_sign_extension: false,
            enable_nontrapping_f2i: false,
            enable_export_memory: false,
            flag_use: HashMap::new(),
            trap_on_abort: false,
            runtime: super::RuntimeStrategy::Incremental,
            source: "".to_string(),
        };

        assert!(
            options
                .to_npx_command(&source_path, &output_path)
                .contains(concat!(
                    "-o \"path/to/output\" ",
                    "-O2 ",
                    "--disable bulk-memory ",
                    "--disable sign-extension ",
                    "--disable nontrapping-f2i ",
                    "--runtime incremental ",
                    "--noExportMemory ",
                )),
        );
    }
}
