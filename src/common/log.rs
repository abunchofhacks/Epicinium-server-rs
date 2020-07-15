/* Log */

pub fn start(logname: &str) -> Result<(), fern::InitError>
{
	let tracelogfilename =
		format_args!("logs/{}.trace.log", logname).to_string();
	let infologfilename = format_args!("logs/{}.info.log", logname).to_string();
	let errorlogfilename =
		format_args!("logs/{}.error.log", logname).to_string();

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
		.level(log::LevelFilter::Trace)
		.chain(fern::log_file(tracelogfilename)?)
		.chain(
			fern::Dispatch::new()
				.level(log::LevelFilter::Debug)
				.chain(fern::log_file(infologfilename)?),
		)
		.chain(
			fern::Dispatch::new()
				.level(log::LevelFilter::Warn)
				.chain(fern::log_file(errorlogfilename)?),
		)
		.apply()?;
	Ok(())
}
