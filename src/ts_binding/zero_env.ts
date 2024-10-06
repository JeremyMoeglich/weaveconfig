import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { existsSync, readFileSync } from "node:fs";

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

export function env(): ConfigType {
	if (!config) {
		throw new Error(
			"Config file not found, if ./config.json does not exist, run weaveconfig, if it does this is likely a bundler issue with import.meta.url or a broken config file",
		);
	}

	return config;
}
