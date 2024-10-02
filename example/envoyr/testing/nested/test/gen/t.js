import dotenv from "dotenv";

dotenv.config();

import fs from "fs";

const env = fs.readFileSync(".env", "utf8");

console.log(
  Object.fromEntries(
    Object.entries(process.env).filter(([key, value]) => env.includes(key))
  )
);
