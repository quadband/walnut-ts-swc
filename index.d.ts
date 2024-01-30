declare function walnutParse(code: string, id: string, walnutKey: string, resolverFn?: ResolverFn): string;
type ResolverFn = (label: string) => string;

export { walnutParse };
