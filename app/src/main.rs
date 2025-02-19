use comtrya_lib::contexts::build_contexts;
use comtrya_lib::contexts::Contexts;
use comtrya_lib::manifests;
use structopt::StructOpt;
use tracing::{error, Level, Subscriber};
use tracing_subscriber::FmtSubscriber;

mod commands;
mod config;
use config::{load_config, Config};

#[derive(Clone, Debug, structopt::StructOpt)]
#[structopt(name = "comtrya")]
pub(crate) struct GlobalArgs {
    /// Directory
    #[structopt(short = "d", long)]
    pub manifest_directory: Option<String>,

    /// Disable color printing
    #[structopt(long = "no-color")]
    pub no_color: bool,

    /// Debug & tracing mode (-v, -vv)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    pub verbose: u8,

    #[structopt(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Debug, StructOpt)]
pub(crate) enum Commands {
    #[structopt(aliases = &["do", "run"])]
    Apply(commands::Apply),
    Version(commands::Version),
}

#[derive(Clone, Debug)]
pub(crate) struct Runtime {
    pub(crate) args: GlobalArgs,
    pub(crate) config: Config,
    pub(crate) contexts: Contexts,
}

pub(crate) fn execute(runtime: Runtime) -> anyhow::Result<()> {
    match &runtime.args.command {
        Commands::Apply(a) => commands::apply(a, &runtime),
        Commands::Version(a) => commands::version(a, &runtime),
    }
}

fn configure_subscriber(args: &GlobalArgs) -> impl Subscriber {
    let builder = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(!args.no_color)
        .with_target(false)
        .without_time();

    match args.verbose {
        0 => builder,
        1 => builder.with_max_level(Level::DEBUG),
        _ => builder.with_max_level(Level::TRACE),
    }
    .finish()
}

#[paw::main]
fn main(args: GlobalArgs) -> anyhow::Result<()> {
    let subscriber = configure_subscriber(&args);

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let config = match load_config(args.clone()) {
        Ok(config) => config,
        Err(error) => {
            error!("{}", error.to_string());
            panic!();
        }
    };

    if !config.disable_update_check {
        check_for_updates(args.no_color);
    }

    // Run Context Providers
    let contexts = build_contexts(&config);

    let runtime = Runtime {
        args,
        config,
        contexts,
    };

    execute(runtime)?;

    Ok(())
}

fn check_for_updates(no_color: bool) {
    use colored::*;
    use update_informer::{registry, Check};

    if no_color {
        control::set_override(false);
    }

    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    let informer = update_informer::new(registry::Crates, pkg_name, pkg_version);

    if let Some(new_version) = informer.check_version().ok().flatten() {
        let msg = format!(
            "A new version of {pkg_name} is available: v{pkg_version} -> {new_version}",
            pkg_name = pkg_name.italic().cyan(),
            new_version = new_version.to_string().green()
        );

        let release_url =
            format!("https://github.com/{pkg_name}/{pkg_name}/releases/tag/{new_version}").blue();
        let changelog = format!("Changelog: {release_url}",);

        let cmd = format!(
            "Run to update: {cmd}",
            cmd = "curl -fsSL https://get.comtrya.dev | sh".green()
        );

        println!("\n{msg}\n{changelog}\n{cmd}");
    }
}
