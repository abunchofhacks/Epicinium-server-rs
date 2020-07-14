/* Log */

pub fn start() -> Result<(), fern::InitError>
{
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
		.chain(fern::log_file("logs/test.log")?)
		.apply()?;
	Ok(())
}
