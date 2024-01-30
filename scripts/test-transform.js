import * as fs from "fs";
import { walnutParse } from "../index.mjs";

const id = "/Users/david/Documents/Projects/walnut-app/src/App.tsx";

const code = fs.readFileSync(id, "utf8");
const walnutKey = "test";

const resolverFn = (label) => {
    return "'example resolve'";
};

const parsed = walnutParse(code, id, walnutKey, resolverFn);

console.log(parsed);