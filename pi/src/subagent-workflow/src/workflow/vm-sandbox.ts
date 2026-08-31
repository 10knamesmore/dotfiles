import vm from "node:vm";

const CONTRACT = "Workflow determinism requires this value to come through args for timestamps or randomness";
const BRIDGE_KEY = "__workflowHostBridge__";

type BranchKind = "parallel" | "pipeline";

export interface WorkflowSandboxInput {
  body: string;
  name: string;
  argsJson: string;
  synchronousTimeoutMs: number;
}

export interface SandboxHostBridge {
  agent: (prompt: unknown, options: unknown) => Promise<string | undefined>;
  phase: (title: unknown) => void;
  log: (message: unknown) => void;
  beginBranchGroup: (kind: BranchKind) => number;
  endBranchGroup: () => void;
  runBranch: (kind: BranchKind, operation: number, branch: number, thunk: () => unknown) => unknown;
}

/** Build and run the constrained realm. Worker lifecycle stays in vm.ts. */
export async function runInConstrainedContext(
  data: WorkflowSandboxInput,
  bridge: SandboxHostBridge,
): Promise<unknown> {
  const context = vm.createContext({}, {
    name: `workflow:${data.name}`,
    codeGeneration: { strings: false, wasm: false },
  });

  // This is the only worker-realm value placed in the context. The bootstrap
  // captures it in a lexical closure and deletes the global before workflow
  // code can run.
  Object.defineProperty(context, BRIDGE_KEY, { value: bridge, configurable: true });

  const bootstrap = `
    "use strict";
    (() => {
      const bridge = globalThis.${BRIDGE_KEY};
      delete globalThis.${BRIDGE_KEY};
      const contract = ${JSON.stringify(CONTRACT)};
      const blocked = (feature) => function unavailable() {
        throw new Error(feature + " is unavailable. " + contract);
      };
      const blockedObject = (feature) => new Proxy(blocked(feature), {
        get: blocked(feature),
        apply: blocked(feature),
        construct: blocked(feature),
      });
      const forward = (call) => (...values) => {
        try {
          return call(...values);
        } catch (error) {
          throw new Error(String(error && error.message || error));
        }
      };

      const RealArray = Array;
      const RealArrayPrototype = Array.prototype;
      const RealObjectPrototype = Object.prototype;
      const RealPromise = Promise;
      const arrayIsArray = Array.isArray;
      const getPrototypeOf = Object.getPrototypeOf;
      const getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
      const ownKeys = Reflect.ownKeys;
      const defineProperty = Object.defineProperty;
      const freeze = Object.freeze;
      const isFiniteNumber = Number.isFinite;
      const isSafeInteger = Number.isSafeInteger;
      const reflectApply = Reflect.apply;
      const promiseCatch = Promise.prototype.catch;
      const promiseThen = Promise.prototype.then;
      const defineArrayElement = (array, index, value) => {
        defineProperty(array, String(index), {
          value,
          writable: true,
          enumerable: true,
          configurable: true,
        });
      };
      const snapshotOrdinaryArray = (value, label) => {
        if (!arrayIsArray(value) || getPrototypeOf(value) !== RealArrayPrototype) {
          throw new TypeError(label + " must be an ordinary array");
        }
        const lengthDescriptor = getOwnPropertyDescriptor(value, "length");
        const length = lengthDescriptor && lengthDescriptor.value;
        if (!isSafeInteger(length) || length < 0) {
          throw new TypeError(label + " has an invalid length");
        }
        const snapshot = new RealArray(length);
        for (let index = 0; index < length; index += 1) {
          const descriptor = getOwnPropertyDescriptor(value, String(index));
          if (!descriptor || !("value" in descriptor)) {
            throw new TypeError(label + " must be dense and contain data elements only");
          }
          defineArrayElement(snapshot, index, descriptor.value);
        }
        return freeze(snapshot);
      };
      const snapshotFunctions = (value, label) => {
        const snapshot = snapshotOrdinaryArray(value, label);
        for (let index = 0; index < snapshot.length; index += 1) {
          if (typeof snapshot[index] !== "function") {
            throw new TypeError(label + " must contain functions only");
          }
        }
        return snapshot;
      };
      const readConcurrency = (value, label, branchCount) => {
        if (value === undefined) return branchCount > 0 ? branchCount : 1;
        if (value === null || typeof value !== "object" || getPrototypeOf(value) !== RealObjectPrototype) {
          throw new TypeError(label + " must be an ordinary object with only a concurrency property");
        }
        const keys = ownKeys(value);
        for (let index = 0; index < keys.length; index += 1) {
          if (keys[index] !== "concurrency") {
            throw new TypeError(label + " supports only the concurrency property");
          }
        }
        const descriptor = getOwnPropertyDescriptor(value, "concurrency");
        if (!descriptor) return branchCount > 0 ? branchCount : 1;
        if (!("value" in descriptor)) throw new TypeError(label + ".concurrency must be a data property");
        const concurrency = descriptor.value;
        if (!isSafeInteger(concurrency) || concurrency < 1) {
          throw new TypeError(label + ".concurrency must be a positive safe integer");
        }
        return concurrency;
      };

      globalThis.agent = (prompt, options) => {
        let raw;
        try {
          raw = bridge.agent(prompt, options);
        } catch (error) {
          throw new Error(String(error && error.message || error));
        }
        // Settle a VM-realm Promise from the hidden host Promise. Returning
        // the host Promise directly would expose its host Function constructor.
        const promise = new RealPromise((resolve, reject) => {
          raw.then(
            (json) => {
              try {
                resolve(json === undefined ? undefined : JSON.parse(json));
              } catch (error) {
                reject(error);
              }
            },
            (error) => reject(new Error(String(error && error.message || error))),
          );
        });
        // A deliberately unconsumed call must not become an unhandled worker
        // rejection. The original Promise still rejects for real consumers.
        void reflectApply(promiseCatch, promise, [() => {}]);
        return promise;
      };
      globalThis.phase = (title) => {
        if (typeof title !== "string") throw new TypeError("phase(title) requires a string");
        return forward(bridge.phase)(title);
      };
      globalThis.log = forward(bridge.log);
      globalThis.console = Object.freeze({
        log: (...values) => globalThis.log(values.map(String).join(" ")),
      });
      globalThis.process = blockedObject("process");
      globalThis.require = blocked("require");
      globalThis.fetch = blocked("fetch");
      globalThis.setTimeout = blocked("setTimeout");
      globalThis.setInterval = blocked("setInterval");
      globalThis.Buffer = blockedObject("Buffer");
      globalThis.SharedArrayBuffer = blockedObject("SharedArrayBuffer");
      globalThis.Atomics = blockedObject("Atomics");
      globalThis.WebAssembly = blockedObject("WebAssembly");
      globalThis.ShadowRealm = blockedObject("ShadowRealm");
      globalThis.Intl = blockedObject("Intl");
      globalThis.WeakRef = blockedObject("WeakRef");
      globalThis.FinalizationRegistry = blockedObject("FinalizationRegistry");
      globalThis.Proxy = blockedObject("Proxy");
      const errorConstructors = [Error, TypeError, RangeError, SyntaxError, EvalError, ReferenceError, URIError, AggregateError];
      for (const ErrorConstructor of errorConstructors) {
        defineProperty(ErrorConstructor, "stackTraceLimit", {
          value: 0,
          writable: false,
          configurable: false,
        });
        defineProperty(ErrorConstructor, "prepareStackTrace", {
          value: () => undefined,
          writable: false,
          configurable: false,
        });
      }
      defineProperty(Error, "captureStackTrace", {
        value: blocked("Error.captureStackTrace"),
        writable: false,
        configurable: false,
      });
      globalThis.args = deepFreeze(JSON.parse(${JSON.stringify(data.argsJson)}));
      const observeBranch = async (branch) => {
        try {
          return { accepted: true, value: await branch };
        } catch (error) {
          return { accepted: false, error };
        }
      };
      const rejectedBranch = async (error) => { throw error; };
      const runBoundedBranches = (count, concurrency, launch) => new RealPromise((resolve) => {
        const outcomes = new RealArray(count);
        if (count === 0) {
          resolve(outcomes);
          return;
        }
        let next = 0;
        let active = 0;
        let settled = 0;
        const admit = () => {
          while (active < concurrency && next < count) {
            const branch = next++;
            active += 1;
            let observed;
            try {
              observed = observeBranch(launch(branch));
            } catch (error) {
              observed = observeBranch(rejectedBranch(error));
            }
            reflectApply(promiseThen, observed, [(outcome) => {
              defineArrayElement(outcomes, branch, outcome);
              active -= 1;
              settled += 1;
              if (settled === count) resolve(outcomes);
              else admit();
            }]);
          }
        };
        admit();
      });
      const settleBranches = async (branches) => {
        const values = new RealArray(branches.length);
        let rejected = false;
        let firstRejection;
        for (let index = 0; index < branches.length; index += 1) {
          const outcome = await branches[index];
          if (outcome.accepted) {
            defineArrayElement(values, index, outcome.value);
          } else {
            if (!rejected) {
              rejected = true;
              firstRejection = outcome.error;
            }
          }
        }
        if (rejected) throw firstRejection;
        return values;
      };
      globalThis.parallel = async (thunks, options) => {
        let began = false;
        try {
          const branchThunks = snapshotFunctions(thunks, "parallel(thunks)");
          const concurrency = readConcurrency(options, "parallel options", branchThunks.length);
          const operation = bridge.beginBranchGroup("parallel");
          began = true;
          const branches = await runBoundedBranches(branchThunks.length, concurrency, (branch) => {
            const thunk = branchThunks[branch];
            return bridge.runBranch("parallel", operation, branch, thunk);
          });
          return await settleBranches(branches);
        } catch (error) {
          throw new Error(String(error && error.message || error));
        } finally {
          if (began) bridge.endBranchGroup();
        }
      };
      globalThis.pipeline = async (items, optionsOrStage, ...remainingStages) => {
        let began = false;
        try {
          const branchItems = snapshotOrdinaryArray(items, "pipeline(items)");
          const hasOptions = optionsOrStage !== undefined && typeof optionsOrStage !== "function";
          const stages = new RealArray(remainingStages.length + (hasOptions || optionsOrStage === undefined ? 0 : 1));
          let stageOffset = 0;
          if (!hasOptions && optionsOrStage !== undefined) {
            defineArrayElement(stages, 0, optionsOrStage);
            stageOffset = 1;
          }
          for (let index = 0; index < remainingStages.length; index += 1) {
            defineArrayElement(stages, stageOffset + index, remainingStages[index]);
          }
          const stageFunctions = snapshotFunctions(stages, "pipeline stages");
          const concurrency = readConcurrency(hasOptions ? optionsOrStage : undefined, "pipeline options", branchItems.length);
          const operation = bridge.beginBranchGroup("pipeline");
          began = true;
          const branches = await runBoundedBranches(branchItems.length, concurrency, (branch) => {
            const item = branchItems[branch];
            return bridge.runBranch("pipeline", operation, branch, async () => {
              let previous;
              for (let stage = 0; stage < stageFunctions.length; stage += 1) {
                previous = await stageFunctions[stage](item, previous);
              }
              return previous;
            });
          });
          return await settleBranches(branches);
        } catch (error) {
          throw new Error(String(error && error.message || error));
        } finally {
          if (began) bridge.endBranchGroup();
        }
      };

      Math.random = blocked("Math.random");
      Object.defineProperties(Promise, {
        all: { value: blocked("Promise.all (use parallel or pipeline)"), writable: false, configurable: false },
        allSettled: { value: blocked("Promise.allSettled (use parallel or pipeline)"), writable: false, configurable: false },
        race: { value: blocked("Promise.race"), writable: false, configurable: false },
        any: { value: blocked("Promise.any"), writable: false, configurable: false },
      });
      defineProperty(globalThis, "Promise", {
        value: RealPromise,
        writable: false,
        configurable: false,
      });

      const localeMethods = [
        [String.prototype, ["localeCompare", "toLocaleLowerCase", "toLocaleUpperCase"]],
        [Number.prototype, ["toLocaleString"]],
        [BigInt.prototype, ["toLocaleString"]],
        [Array.prototype, ["toLocaleString"]],
      ];
      for (const [prototype, methods] of localeMethods) {
        for (const method of methods) {
          Object.defineProperty(prototype, method, {
            value: blocked(prototype.constructor.name + ".prototype." + method),
            writable: false,
            configurable: false,
          });
        }
      }
      Object.defineProperty(Object.getPrototypeOf(Uint8Array.prototype), "toLocaleString", {
        value: blocked("TypedArray.prototype.toLocaleString"),
        writable: false,
        configurable: false,
      });

      const RealDate = Date;
      function SafeDate(...values) {
        if (!new.target || values.length !== 1 || typeof values[0] !== "number" || !isFiniteNumber(values[0])) {
          throw new Error("Date construction requires exactly one primitive finite number of epoch milliseconds. " + contract + ". Pass epoch milliseconds via args or use Date.UTC");
        }
        return Reflect.construct(RealDate, values, new.target);
      }
      Object.defineProperties(SafeDate, {
        now: { value: blocked("Date.now"), writable: false, configurable: false },
        parse: { value: blocked("Date.parse"), writable: false, configurable: false },
        UTC: { value: RealDate.UTC, writable: false, configurable: false },
      });
      SafeDate.prototype = RealDate.prototype;
      Object.defineProperty(RealDate.prototype, "constructor", {
        value: SafeDate,
        writable: false,
        configurable: false,
      });
      const localDateMethods = [
        "getDate", "getDay", "getFullYear", "getHours", "getMilliseconds", "getMinutes", "getMonth",
        "getSeconds", "getTimezoneOffset", "getYear", "setDate", "setFullYear", "setHours",
        "setMilliseconds", "setMinutes", "setMonth", "setSeconds", "setYear", "toDateString",
        "toLocaleDateString", "toLocaleString", "toLocaleTimeString", "toString", "toTimeString",
      ];
      for (const method of localDateMethods) {
        Object.defineProperty(RealDate.prototype, method, {
          value: blocked("Date.prototype." + method),
          writable: false,
          configurable: false,
        });
      }
      Object.freeze(RealDate.prototype);
      Object.freeze(SafeDate);
      globalThis.Date = SafeDate;

      function deepFreeze(value) {
        if (value && typeof value === "object" && !Object.isFrozen(value)) {
          Object.freeze(value);
          for (const key of Object.keys(value)) deepFreeze(value[key]);
        }
        return value;
      }
    })();
  `;
  new vm.Script(bootstrap, { filename: `${data.name}.bootstrap.js` }).runInContext(context, {
    timeout: data.synchronousTimeoutMs,
  });
  if (Reflect.has(context, BRIDGE_KEY)) throw new Error("Workflow VM bootstrap failed to hide its host bridge");

  const wrapped = `"use strict";\n(async () => {\n${data.body}\n})()`;
  return new vm.Script(wrapped, { filename: `${data.name}.workflow.js` }).runInContext(context, {
    timeout: data.synchronousTimeoutMs,
  });
}
