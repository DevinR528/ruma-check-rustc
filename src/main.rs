#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_attr;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_hir_pretty;
extern crate rustc_index;
extern crate rustc_infer;
extern crate rustc_interface;
extern crate rustc_lexer;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_mir;
extern crate rustc_parse;
extern crate rustc_parse_format;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_trait_selection;
extern crate rustc_typeck;

use rustc_driver::{Callbacks, Compilation};
use rustc_interface::{
    interface::{Compiler, Config},
    Queries,
};
use rustc_middle::ty::TyCtxt;

use std::{
    env,
    ops::Deref,
    path::{Path, PathBuf},
    process::{exit, Command},
};

mod config;
mod lints;
mod utils;

const VERSION: &str = "0.1.0";

struct DefaultCallbacks;
impl Callbacks for DefaultCallbacks {}

struct RumaCallbacks;
impl Callbacks for RumaCallbacks {
    fn config(&mut self, config: &mut Config) {
        let previous = config.register_lints.take();
        config.register_lints = Some(Box::new(move |sess, mut lint_store| {
            // technically we're ~guaranteed that this is none but might as well call anything that
            // is there already. Certainly it can't hurt.
            if let Some(previous) = &previous {
                (previous)(sess, lint_store);
            }

            let conf = config::read_conf(&[], &sess).unwrap();
            lints::register_plugins(&mut lint_store, &sess, &conf);
        }));

        // FIXME: #4825; This is required, because Clippy lints that are based on MIR have to be
        // run on the unoptimized MIR. On the other hand this results in some false negatives. If
        // MIR passes can be enabled / disabled separately, we should figure out, what passes to
        // use for Clippy.
        config.opts.debugging_opts.mir_opt_level = 0;
    }

    /// We must go to the next stage of compilation in order to run LateLintPass lints.
    fn after_parsing<'tcx>(
        &mut self,
        _compiler: &Compiler,
        _queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        Compilation::Continue
    }

    /// Keep on truckin'.
    fn after_expansion<'tcx>(
        &mut self,
        _compiler: &Compiler,
        _queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        Compilation::Continue
    }

    /// We stop after analysis since we don't want to produce binary.
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &Compiler,
        _queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        Compilation::Stop
    }
}

fn toolchain_path(home: Option<String>, toolchain: Option<String>) -> Option<PathBuf> {
    home.and_then(|home| {
        toolchain.map(|toolchain| {
            let mut path = PathBuf::from(home);
            path.push("toolchains");
            path.push(toolchain);
            path
        })
    })
}

/// If a command-line option matches `find_arg`, then apply the predicate `pred` on its value. If
/// true, then return it. The parameter is assumed to be either `--arg=value` or `--arg value`.
fn arg_value<'a, T: Deref<Target = str>>(
    args: &'a [T],
    find_arg: &str,
    pred: impl Fn(&str) -> bool,
) -> Option<&'a str> {
    let mut args = args.iter().map(Deref::deref);
    while let Some(arg) = args.next() {
        let mut arg = arg.splitn(2, '=');
        if arg.next() != Some(find_arg) {
            continue;
        }

        match arg.next().or_else(|| args.next()) {
            Some(v) if pred(v) => return Some(v),
            _ => {}
        }
    }
    None
}

fn display_help() {
    println!(
        "\
Checks a package to catch common mistakes and improve your Rust code.
Usage:
    cargo clippy [options] [--] [<opts>...]
Common options:
    -h, --help               Print this message
        --rustc              Pass all args to rustc
    -V, --version            Print version info and exit
Other options are the same as `cargo check`.
To allow or deny a lint from the command line you can use `cargo clippy --`
with:
    -W --warn OPT       Set lint warnings
    -A --allow OPT      Set lint allowed
    -D --deny OPT       Set lint denied
    -F --forbid OPT     Set lint forbidden
You can use tool lints to allow or deny lints from your code, eg.:
    #[allow(clippy::needless_lifetimes)]
"
    );
}

fn main() {
    rustc_driver::init_rustc_env_logger();
    exit(rustc_driver::catch_with_exit_code(move || {
        let mut orig_args: Vec<String> = env::args().collect();

        // Get the sysroot, looking from most specific to this invocation to the least:
        // - command line
        // - runtime environment
        //    - SYSROOT
        //    - RUSTUP_HOME, MULTIRUST_HOME, RUSTUP_TOOLCHAIN, MULTIRUST_TOOLCHAIN
        // - sysroot from rustc in the path
        // - compile-time environment
        //    - SYSROOT
        //    - RUSTUP_HOME, MULTIRUST_HOME, RUSTUP_TOOLCHAIN, MULTIRUST_TOOLCHAIN
        let sys_root_arg = arg_value(&orig_args, "--sysroot", |_| true);
        let have_sys_root_arg = sys_root_arg.is_some();
        let sys_root = sys_root_arg
            .map(PathBuf::from)
            .or_else(|| std::env::var("SYSROOT").ok().map(PathBuf::from))
            .or_else(|| {
                let home = std::env::var("RUSTUP_HOME")
                    .or_else(|_| std::env::var("MULTIRUST_HOME"))
                    .ok();
                let toolchain = std::env::var("RUSTUP_TOOLCHAIN")
                    .or_else(|_| std::env::var("MULTIRUST_TOOLCHAIN"))
                    .ok();
                toolchain_path(home, toolchain)
            })
            .or_else(|| {
                Command::new("rustc")
                    .arg("--print")
                    .arg("sysroot")
                    .output()
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                    .map(|s| PathBuf::from(s.trim()))
            })
            .or_else(|| option_env!("SYSROOT").map(PathBuf::from))
            .or_else(|| {
                let home = option_env!("RUSTUP_HOME")
                    .or(option_env!("MULTIRUST_HOME"))
                    .map(ToString::to_string);
                let toolchain = option_env!("RUSTUP_TOOLCHAIN")
                    .or(option_env!("MULTIRUST_TOOLCHAIN"))
                    .map(ToString::to_string);
                toolchain_path(home, toolchain)
            })
            .map(|pb| pb.to_string_lossy().to_string())
            .expect("need to specify SYSROOT env var during clippy compilation, or use rustup or multirust");

        // Make this work like a subcommand that passes further args to "rustc"
        // for example `ruma-check --rustc --version` will print the rustc version.
        if let Some(pos) = orig_args.iter().position(|arg| arg == "--rustc") {
            orig_args.remove(pos);
            orig_args[0] = "rustc".to_string();

            // if we call "rustc", we need to pass --sysroot here as well
            let mut args: Vec<String> = orig_args.clone();
            if !have_sys_root_arg {
                args.extend(vec!["--sysroot".into(), sys_root]);
            };

            return rustc_driver::RunCompiler::new(&args, &mut DefaultCallbacks).run();
        }

        if orig_args.iter().any(|a| a == "--version" || a == "-V") {
            println!("{}", VERSION);
            exit(0);
        }

        if orig_args.iter().any(|a| a == "--help" || a == "-h") {
            display_help();
            exit(0);
        }

        // this conditional check for the --sysroot flag is there so users can call
        // `clippy_driver` directly
        // without having to pass --sysroot or anything
        let mut args: Vec<String> = orig_args.clone();
        if !have_sys_root_arg {
            args.extend(vec!["--sysroot".into(), sys_root]);
        };

        let mut ruma = RumaCallbacks;

        rustc_driver::RunCompiler::new(&args, &mut ruma).run()
    }))
}
