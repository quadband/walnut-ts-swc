import * as fs from "fs";
import { walnutParse } from "../index.mjs";

const testFiles = [
    {
        id: "/Users/david/Documents/Projects/walnut-app/src/App.tsx",
        code: fs.readFileSync("/Users/david/Documents/Projects/walnut-app/src/App.tsx", "utf8")
    },
    {
        id: "/Users/david/Documents/Projects/walnut-app/src/index.tsx",
        code: fs.readFileSync("/Users/david/Documents/Projects/walnut-app/src/index.tsx", "utf8")
    }
];

const walnutKey = "test";
const resolverFn = (label) => {
    return "'example resolve'";
};

for(let i=0, len=testFiles.length;i<len;i++){
    const start = performance.now();
    const parsed = walnutParse(
        testFiles[i].code, 
        testFiles[i].id, 
        walnutKey, 
        resolverFn
    );
    //console.log(parsed);
    console.log(testFiles[i].id);
    console.log("Time:", performance.now() - start,"ms");
}
