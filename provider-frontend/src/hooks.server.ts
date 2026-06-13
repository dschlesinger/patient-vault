import fs from 'fs';
import path from 'path';
import { env } from '$env/dynamic/private';
import type { Handle } from '@sveltejs/kit';

const logFolder = env.LOG_FOLDER;

function ensureLogDir() {
	if (!logFolder) return;
	fs.mkdirSync(logFolder, { recursive: true });
}

function writeLog(line: string) {
	if (!logFolder) return;
	ensureLogDir();
	fs.appendFileSync(path.join(logFolder, 'frontend.log'), line);
}

// Patch console methods to tee into frontend.log when LOG_FOLDER is set.
// Guard against double-patching on HMR reloads.
if (logFolder && !(console.log as unknown as { __patched?: boolean }).__patched) {
	ensureLogDir();
	const stream = fs.createWriteStream(path.join(logFolder, 'frontend.log'), { flags: 'a' });
	stream.on('error', (err) => {
		process.stderr.write(`[patient-vault] failed to write frontend.log: ${err.message}\n`);
	});

	const fmt = (level: string, args: unknown[]) =>
		`[${new Date().toISOString()}] [${level}] ${args
			.map((a) => (typeof a === 'string' ? a : JSON.stringify(a)))
			.join(' ')}\n`;

	const wrap =
		(level: string, orig: (...a: unknown[]) => void) =>
		(...args: unknown[]) => {
			orig(...args);
			stream.write(fmt(level, args));
		};

	console.log = wrap('LOG', console.log.bind(console));
	console.info = wrap('INFO', console.info.bind(console));
	console.warn = wrap('WARN', console.warn.bind(console));
	console.error = wrap('ERROR', console.error.bind(console));
	(console.log as unknown as { __patched: boolean }).__patched = true;

	console.info(`Provider frontend logging enabled → ${logFolder}/frontend.log`);
}

export const handle: Handle = async ({ event, resolve }) => {
	const start = Date.now();
	const response = await resolve(event);
	const ms = Date.now() - start;
	const line = `[${new Date().toISOString()}] [REQUEST] ${event.request.method} ${event.url.pathname} → ${response.status} (${ms}ms)\n`;
	writeLog(line);
	return response;
};
