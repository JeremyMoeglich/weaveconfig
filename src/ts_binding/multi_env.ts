import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";
import { existsSync, readFileSync, realpathSync, statSync } from "node:fs";

// https://github.com/motdotla/dotenv/blob/master/lib/main.js
// https://github.com/motdotla/dotenv/blob/master/LICENSE
const LINE =
	/(?:^|^)\s*(?:export\s+)?([\w.-]+)(?:\s*=\s*?|:\s+?)(\s*'(?:\\'|[^'])*'|\s*"(?:\\"|[^"])*"|\s*`(?:\\`|[^`])*`|[^#\r\n]+)?\s*(?:#.*)?(?:$|$)/gm;
function parse(src: string): Record<string, string> {
	const obj: Record<string, string> = {};

	// Convert line breaks to same format
	const lines = src.replace(/\r\n?/gm, "\n");
	while (true) {
		const match = LINE.exec(lines);
		if (match === null) break;

		const key = match[1];

		// Default undefined or null to empty string
		let value = match[2] || "";

		// Remove whitespace
		value = value.trim();

		// Check if double quoted
		const maybeQuote = value[0];

		// Remove surrounding quotes
		value = value.replace(/^(['"`])([\s\S]*)\1$/gm, "$2");

		// Expand newlines if double quoted
		if (maybeQuote === '"') {
			value = value.replace(/\\n/g, "\n");
			value = value.replace(/\\r/g, "\r");
		}

		// Add to object
		obj[key] = value;
	}

	return obj;
}

/**
 * Finds all `.env` files in the current and parent directories.
 * Prevents infinite loops by tracking visited directories using their real paths.
 *
 * @param startDir - The directory to start searching from. Defaults to the current working directory.
 * @returns An array of absolute paths to `.env` files found. Ordered from closest to farthest.
 */
function findEnvFiles(startDir: string): string[] {
	const envFiles: string[] = [];
	const visitedDirs: Set<string> = new Set();
	let currentDir = resolve(startDir);

	while (true) {
		let realCurrentDir: string;
		try {
			realCurrentDir = realpathSync(currentDir);
		} catch (err) {
			console.warn(
				`Warning: Unable to resolve real path for directory "${currentDir}". Skipping.`,
				err,
			);
			break;
		}

		if (visitedDirs.has(realCurrentDir)) {
			console.warn(
				`Detected a symlink loop at "${realCurrentDir}". Stopping search to prevent infinite loop.`,
			);
			break;
		}

		visitedDirs.add(realCurrentDir);

		const envFilePath = join(currentDir, ".env");
		try {
			const stat = statSync(envFilePath);
			if (stat.isFile()) {
				envFiles.push(envFilePath);
			}
		} catch (err) {
			// If the file does not exist, ignore and continue
			if ((err as NodeJS.ErrnoException).code !== "ENOENT") {
				console.warn(`Warning: Unable to access "${envFilePath}".`, err);
			}
		}

		const parentDir = dirname(currentDir);
		if (parentDir === currentDir) {
			// Reached the root directory
			break;
		}
		currentDir = parentDir;
	}

	return envFiles;
}

function load_env_variable(
	dotenv_start_dir: string,
	key: string,
): string | undefined {
	// check if the key is already set
	if (process.env[key]) {
		return process.env[key];
	}

	// check if the key is in the .env file
	const envFiles = findEnvFiles(dotenv_start_dir);
	for (const envFile of envFiles) {
		const file = readFileSync(envFile, "utf8");
		const env = parse(file);
		if (env[key]) {
			return env[key];
		}
	}
	return undefined;
}

// Get the current file URL and convert it to a path
const __filename = fileURLToPath(import.meta.url);
// Get the directory name of the current file
const __dirname = dirname(__filename);

function locate_config_file(): string | null {
	const name = "config.json";
	const path = join(__dirname, name);
	if (existsSync(path)) {
		return path;
	}
	return null;
}

function read_config_file(): ConfigType | null {
	const path = locate_config_file();
	if (!path) {
		return null;
	}
	const file = readFileSync(path, "utf8");
	return JSON.parse(file);
}

const config = read_config_file();
const used_environment = load_env_variable(__dirname, "ENV");

export function env<T extends (typeof environments)[number]>(
	use_environment: T | undefined = used_environment as T | undefined,
): ConfigType & ConfigType[T] {
	if (!use_environment) {
		throw new Error(
			`ENV is not set, please set ENV variable to one of ${environments.join(
				", ",
			)}, you can pass this variable directly or in a .env file`,
		);
	}
	if (!environments.includes(use_environment)) {
		throw new Error(`ENV must be one of ${environments.join(", ")}`);
	}

	if (!config) {
		throw new Error(
			"Config file not found, if ./config.json does not exist, run weaveconfig, if it does this is likely a bundler issue with import.meta.url",
		);
	}

	return {
		...config,
		...config[use_environment],
	};
}
