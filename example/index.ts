import { env } from "./gen/binding";

console.log(env("prod").variable1);
