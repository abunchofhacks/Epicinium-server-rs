/* Log */

use epicinium_lib;

use serde_derive::{Deserialize, Serialize};

pub fn trace_filename(logname: &str) -> String
{
	format!("logs/{}.trace.log", logname).to_string()
}

pub fn info_filename(logname: &str) -> String
{
	format!("logs/{}.info.log", logname).to_string()
}

pub fn error_filename(logname: &str) -> String
{
	format!("logs/{}.error.log", logname).to_string()
}

pub fn start(logname: &str, level: Level) -> Result<(), fern::InitError>
{
	let tracelogfilename: std::path::PathBuf = trace_filename(logname).into();
	let infologfilename: std::path::PathBuf = info_filename(logname).into();
	let errorlogfilename: std::path::PathBuf = error_filename(logname).into();
	let sighup = Some(libc::SIGHUP);

	let levelfilter = match level
	{
		Level::Error => log::LevelFilter::Error,
		Level::Warn => log::LevelFilter::Warn,
		Level::Info => log::LevelFilter::Info,
		Level::Debug => log::LevelFilter::Debug,
		Level::Verbose => log::LevelFilter::Trace,
	};

	fern::Dispatch::new()
		.level(levelfilter)
		.filter(|metadata| {
			// Smaller is more severe.
			metadata.level() <= log::LevelFilter::Info
				|| !matches_blacklist(metadata.target())
		})
		.format(|out, message, record| {
			out.finish(format_args!(
				"{time} {lvl:5} [{tid:x}] [{target}.rs:{ln}] {msg}",
				time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f"),
				lvl = record.level(),
				tid = thread_id::get(),
				target = record.target(),
				ln = record.line().unwrap_or(0),
				msg = message
			))
		})
		.chain(fern::log_reopen(&tracelogfilename, sighup)?)
		.chain(
			fern::Dispatch::new()
				.level(log::LevelFilter::Debug)
				.chain(fern::log_reopen(&infologfilename, sighup)?),
		)
		.chain(
			fern::Dispatch::new()
				.level(log::LevelFilter::Warn)
				.chain(fern::log_reopen(&errorlogfilename, sighup)?),
		)
		.apply()?;

	let severity = match level
	{
		Level::Error => epicinium_lib::log::Severity::Error,
		Level::Warn => epicinium_lib::log::Severity::Warning,
		Level::Info => epicinium_lib::log::Severity::Info,
		Level::Debug => epicinium_lib::log::Severity::Debug,
		Level::Verbose => epicinium_lib::log::Severity::Verbose,
	};
	epicinium_lib::log_initialize(severity);

	Ok(())
}

fn matches_blacklist(target: &str) -> bool
{
	target.starts_with("hyper")
		|| target.starts_with("want")
		|| target.starts_with("mio")
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level
{
	Error,
	Warn,
	Info,
	Debug,
	Verbose,
}
