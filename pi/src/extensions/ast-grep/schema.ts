import { Type, type Static } from "typebox";

export const parameters = Type.Object({
  paths: Type.Array(Type.String({ minLength: 1 }), {
    minItems: 1,
    description: "Files or directories, relative to the working directory or absolute.",
  }),
  language: Type.String({ minLength: 1, description: "ast-grep language, e.g. rust, typescript, python." }),
  pattern: Type.Optional(
    Type.String({
      minLength: 1,
      description:
        "Code pattern. $NAME captures one node; $$$NAME captures multiple nodes. Supply exactly one of pattern or rule.",
    }),
  ),
  rule: Type.Optional(
    Type.Record(Type.String(), Type.Unknown(), {
      description:
        "Native ast-grep Rule object, e.g. {pattern: 'foo($X)', inside: {kind: 'function_item', stopBy: 'end'}}. Not a complete YAML rule configuration.",
    }),
  ),
  globs: Type.Optional(
    Type.Array(Type.String(), {
      description: "Include/exclude file globs; prefix exclusions with !. Uses ast-grep's --globs semantics.",
    }),
  ),
  replacement: Type.Optional(
    Type.String({
      description: "Rewrite template using captured variables. Omit to search; an empty string deletes matched nodes.",
    }),
  ),
  preview: Type.Optional(
    Type.Boolean({
      description: "With replacement, defaults to true (show diff only). Set false to write changes.",
    }),
  ),
});

const position = Type.Object({ line: Type.Integer(), column: Type.Integer() });
const range = Type.Object(
  {
    byteOffset: Type.Object({ start: Type.Integer(), end: Type.Integer() }),
    start: position,
    end: position,
  },
  { description: "Native ast-grep range: zero-based positions and UTF-8 byte offsets; end is exclusive." },
);
const capture = Type.Object({ text: Type.String(), range });

export const matchSchema = Type.Object({
  file: Type.String(),
  text: Type.String(),
  range,
  replacement: Type.Optional(Type.String()),
  metaVariables: Type.Optional(
    Type.Object({
      single: Type.Record(Type.String(), capture),
      multi: Type.Record(Type.String(), Type.Array(capture)),
      transformed: Type.Record(Type.String(), Type.String()),
    }),
  ),
});

export const outputSchema = Type.Object({
  mode: Type.Union([Type.Literal("search"), Type.Literal("preview"), Type.Literal("apply")]),
  status: Type.Union([Type.Literal("completed"), Type.Literal("failed"), Type.Literal("cancelled")]),
  matchCount: Type.Integer(),
  files: Type.Array(Type.String()),
  matches: Type.Array(matchSchema),
  output: Type.String({ description: "Search results, native preview diff, or actual applied file diffs." }),
  command: Type.Object({
    exitCode: Type.Integer(),
    stderr: Type.String(),
  }),
  fullOutputPath: Type.Optional(Type.String()),
});

export type AstGrepParameters = Static<typeof parameters>;
export type AstGrepMatch = Static<typeof matchSchema>;
export type AstGrepOutput = Static<typeof outputSchema>;
