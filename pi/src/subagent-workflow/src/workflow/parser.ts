import { parse } from "acorn";
import type { WorkflowMeta } from "../types.js";
import { errorMessage, isRecord } from "../util.js";

interface Node {
  type: string;
  start: number;
  end: number;
  loc?: { start: { line: number } };
  [key: string]: unknown;
}

declare const parsedWorkflowBrand: unique symbol;

type DeepReadonly<T> = T extends readonly (infer Item)[]
  ? readonly DeepReadonly<Item>[]
  : T extends object
    ? { readonly [Key in keyof T]: DeepReadonly<T[Key]> }
    : T;

/** A parse-once workflow artifact branded at the type level. */
export interface ParsedWorkflow {
  readonly [parsedWorkflowBrand]: true;
  /** Exact input text, retained for hashing, persistence, resume, and archive. */
  readonly script: string;
  readonly meta: DeepReadonly<WorkflowMeta>;
  readonly body: string;
  readonly literalModels: readonly string[];
}

export function parseWorkflowScript(script: string): ParsedWorkflow {
  let program: Node;
  try {
    program = parse(script, {
      ecmaVersion: "latest",
      sourceType: "module",
      allowAwaitOutsideFunction: true,
      allowReturnOutsideFunction: true,
      locations: true,
    }) as unknown as Node;
  } catch (error) {
    throw new Error(`Workflow syntax error: ${errorMessage(error)}`);
  }
  const literalModels = new Set<string>();
  inspectProgram(program, literalModels);
  const statements = program.body as Node[];
  const exports = statements.filter(isMetaExport);
  if (exports.length === 0) throw new Error("Workflow meta is missing: expected `export const meta = {...}` at line 1");
  if (exports.length > 1) throw at(exports[1]!, "Workflow meta may only be declared once");
  const statement = exports[0]!;
  const unsupportedModule = statements.find((item) => (item.type === "ImportDeclaration" || item.type.startsWith("Export")) && item !== statement);
  if (unsupportedModule) throw at(unsupportedModule, "Workflow scripts may not import or export anything except literal meta");
  const declaration = statement.declaration as Node;
  const declarations = declaration.declarations as Node[];
  if (declaration.kind !== "const" || declarations.length !== 1) throw at(statement, "Workflow meta must be `export const meta = {...}`");
  const initializer = declarations[0]!.init as Node | undefined;
  if (!initializer) throw at(statement, "Workflow meta must be a literal object");
  let value: unknown;
  try {
    value = literalValue(initializer, "meta");
  } catch (error) {
    throw at(initializer, errorMessage(error));
  }
  validateMeta(value, statement);
  return Object.freeze({
    script,
    meta: deepFreeze(value),
    body: `${script.slice(0, statement.start)}${script.slice(statement.end)}`,
    literalModels: Object.freeze([...literalModels]),
  }) as unknown as ParsedWorkflow;
}

function deepFreeze<T>(value: T): T {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) deepFreeze(child);
  return Object.freeze(value);
}

function isMetaExport(node: Node): boolean {
  if (node.type !== "ExportNamedDeclaration") return false;
  const declaration = node.declaration as Node | undefined;
  if (declaration?.type !== "VariableDeclaration") return false;
  return (declaration.declarations as Node[]).some((item) => (item.id as Node | undefined)?.type === "Identifier" && (item.id as Node).name === "meta");
}

function literalValue(node: Node, path: string): unknown {
  if (node.type === "Literal") return node.value;
  if (node.type === "ArrayExpression") {
    return (node.elements as Array<Node | null>).map((item, index) => {
      if (!item) throw new Error(`${path}[${index}] may not be sparse`);
      return literalValue(item, `${path}[${index}]`);
    });
  }
  if (node.type !== "ObjectExpression") throw new Error(`${path} must contain literal values only`);
  const result: Record<string, unknown> = {};
  for (const property of node.properties as Node[]) {
    if (property.type !== "Property" || property.computed || property.method || property.kind !== "init") {
      throw new Error(`${path} must contain plain, non-computed properties only`);
    }
    const keyNode = property.key as Node;
    const key = keyNode.type === "Identifier" ? String(keyNode.name) : keyNode.type === "Literal" ? String(keyNode.value) : "";
    if (!key || ["__proto__", "constructor", "prototype"].includes(key)) throw new Error(`${path} contains an invalid key`);
    result[key] = literalValue(property.value as Node, `${path}.${key}`);
  }
  return result;
}

function validateMeta(value: unknown, node: Node): asserts value is WorkflowMeta {
  if (!isRecord(value)) throw at(node, "Workflow meta must be a literal object");
  const meta = value;
  const allowed = new Set(["name", "description", "phases"]);
  const extra = Object.keys(meta).find((key) => !allowed.has(key));
  if (extra) throw at(node, `Workflow meta has unknown property "${extra}"`);
  if (typeof meta.name !== "string" || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(meta.name)) throw at(node, "Workflow meta.name must be a kebab-case string");
  if (typeof meta.description !== "string") throw at(node, "Workflow meta.description must be a string");
  if (meta.phases !== undefined) {
    if (!Array.isArray(meta.phases)) throw at(node, "Workflow meta.phases must be an array");
    for (const phase of meta.phases) {
      if (!isRecord(phase)) throw at(node, "Each workflow phase must be an object");
      const item = phase;
      if (typeof item.title !== "string" || (item.detail !== undefined && typeof item.detail !== "string")) throw at(node, "Each workflow phase needs a string title and optional string detail");
      if (Object.keys(item).some((key) => key !== "title" && key !== "detail")) throw at(node, "Workflow phases only support title and detail");
    }
  }
}

function inspectProgram(node: Node, literalModels: Set<string>): void {
  if (node.type === "ImportExpression") throw at(node, "Dynamic import is unavailable in workflows because execution must be deterministic");
  collectLiteralModel(node, literalModels);
  for (const value of Object.values(node)) {
    if (Array.isArray(value)) {
      for (const item of value) {
        if (item && typeof item === "object" && "type" in item) inspectProgram(item as unknown as Node, literalModels);
      }
    } else if (value && typeof value === "object" && "type" in value) {
      inspectProgram(value as Node, literalModels);
    }
  }
}

function collectLiteralModel(node: Node, models: Set<string>): void {
  if (node.type !== "CallExpression") return;
  const callee = node.callee as Node;
  const options = (node.arguments as Node[])[1];
  if (callee.type !== "Identifier" || callee.name !== "agent" || options?.type !== "ObjectExpression") return;
  for (const property of options.properties as Node[]) {
    if (property.type !== "Property" || property.computed) continue;
    const key = property.key as Node;
    const name = key.type === "Identifier" ? key.name : key.type === "Literal" ? key.value : undefined;
    const value = property.value as Node;
    if (name === "model" && value.type === "Literal" && typeof value.value === "string") models.add(value.value);
  }
}

function at(node: Node, message: string): Error {
  return new Error(`${message} at line ${node.loc?.start.line ?? 1}`);
}
