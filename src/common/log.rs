/* Log */

pub fn start(logname: &str) -> Result<(), fern::InitError>
{
	let tracelogfilename: std::path::PathBuf =
		format_args!("logs/{}.trace.log", logname)
			.to_string()
			.into();
	let infologfilename: std::path::PathBuf =
		format_args!("logs/{}.info.log", logname).to_string().into();
	let errorlogfilename: std::path::PathBuf =
		format_args!("logs/{}.error.log", logname)
			.to_string()
			.into();

	let sighup = Some(libc::SIGHUP);

	fern::Dispatch::new()
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
	Ok(())
}
