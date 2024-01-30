import { getHandler } from "./glue.cjs";

export function walnutParse(
  code: string,
  id: string,
  walnutKey: string,
  resolverFn?: ResolverFn
): string {
  const handler = getHandler(code, id, walnutKey);
  handler.run();
  if (handler.needResolver) {
    if (!resolverFn) {
      throw new Error(
        "Walnut encountered a resolver but has no resolver function"
      );
    }

    const resLabels = handler.getResolverLabels();
    const resolved_arr: Array<[string, string]> = [];

    resLabels.forEach((label) => {
      const res = resolverFn(label);
      resolved_arr.push([label, res]);
    });

    handler.satisfyResolvers(resolved_arr);
  }

  return handler.getOutput();
}

export type ResolverFn = (label: string) => string;
