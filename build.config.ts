import { defineBuildConfig } from "unbuild";

export default defineBuildConfig({
  entries: ["./index.ts"],
  clean: false,
  declaration: "compatible",
  outDir: "./",
  dependencies: ["./glue.cjs"],
  externals: ["./glue.cjs"],
  failOnWarn: false,
});
