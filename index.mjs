import { getHandler } from './glue.cjs';

function walnutParse(code, id, walnutKey, resolverFn) {
  const handler = getHandler(code, id, walnutKey);
  handler.run();
  if (handler.needResolver) {
    if (!resolverFn) {
      throw new Error(
        "Walnut encountered a resolver but has no resolver function"
      );
    }
    const resLabels = handler.getResolverLabels();
    const resolved_arr = [];
    resLabels.forEach((label) => {
      const res = resolverFn(label);
      resolved_arr.push([label, res]);
    });
    handler.satisfyResolvers(resolved_arr);
  }
  return handler.getOutput();
}

export { walnutParse };
