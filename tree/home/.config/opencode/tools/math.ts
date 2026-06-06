import { tool } from "@opencode-ai/plugin"

export const calculate = tool({
  description: "Perform mathematical calculation with two numbers",
  args: {
    a: tool.schema.number().describe("First number"),
    b: tool.schema.number().describe("Second number"),
    operation: tool.schema.enum(["+", "-", "*", "/", "**", "%"]).describe("Mathematical operation: +, -, *, /, **, %"),
  },
  async execute(args) {
    let result: number
    switch (args.operation) {
      case "+":
        result = args.a + args.b
        break
      case "-":
        result = args.a - args.b
        break
      case "*":
        result = args.a * args.b
        break
      case "/":
        if (args.b === 0) {
          throw new Error("Division by zero is not allowed")
        }
        result = args.a / args.b
        break
      case "**":
        result = Math.pow(args.a, args.b)
        break
      case "%":
        if (args.b === 0) {
          throw new Error("Modulo by zero is not allowed")
        }
        result = args.a % args.b
        break
      default:
        throw new Error(`Unsupported operation: ${args.operation}`)
    }
    return result.toString()
  },
})

export const advanced = tool({
  description: "Advanced mathematical operations",
  args: {
    operation: tool.schema.enum(["sqrt", "abs", "ceil", "floor", "round", "sin", "cos", "tan", "log", "exp"]).describe("Advanced operation"),
    num: tool.schema.number().describe("Number for operation"),
  },
  async execute(args) {
    let result: number
    switch (args.operation) {
      case "sqrt":
        if (args.num < 0) {
          throw new Error("Cannot calculate square root of negative number")
        }
        result = Math.sqrt(args.num)
        break
      case "abs":
        result = Math.abs(args.num)
        break
      case "ceil":
        result = Math.ceil(args.num)
        break
      case "floor":
        result = Math.floor(args.num)
        break
      case "round":
        result = Math.round(args.num)
        break
      case "sin":
        result = Math.sin(args.num)
        break
      case "cos":
        result = Math.cos(args.num)
        break
      case "tan":
        result = Math.tan(args.num)
        break
      case "log":
        if (args.num <= 0) {
          throw new Error("Logarithm is not defined for non-positive numbers")
        }
        result = Math.log(args.num)
        break
      case "exp":
        result = Math.exp(args.num)
        break
      default:
        throw new Error(`Unsupported operation: ${args.operation}`)
    }
    return result.toString()
  },
})

export const statistics = tool({
  description: "Statistical calculations",
  args: {
    operation: tool.schema.enum(["sum", "average", "min", "max", "median", "variance", "stddev"]).describe("Statistical operation"),
    numbers: tool.schema.array(tool.schema.number()).describe("Array of numbers"),
  },
  async execute(args) {
    if (args.numbers.length === 0) {
      throw new Error("Cannot perform operation on empty array")
    }

    let result: number
    switch (args.operation) {
      case "sum":
        result = args.numbers.reduce((acc, num) => acc + num, 0)
        break
      case "average":
        result = args.numbers.reduce((acc, num) => acc + num, 0) / args.numbers.length
        break
      case "min":
        result = Math.min(...args.numbers)
        break
      case "max":
        result = Math.max(...args.numbers)
        break
      case "median": {
        const sorted = [...args.numbers].sort((a, b) => a - b)
        const middle = Math.floor(sorted.length / 2)
        if (sorted.length % 2 === 0) {
          result = (sorted[middle - 1] + sorted[middle]) / 2
        } else {
          result = sorted[middle]
        }
        break
      }
      case "variance": {
        const avg = args.numbers.reduce((acc, num) => acc + num, 0) / args.numbers.length
        result = args.numbers.reduce((acc, num) => acc + Math.pow(num - avg, 2), 0) / args.numbers.length
        break
      }
      case "stddev": {
        const avg = args.numbers.reduce((acc, num) => acc + num, 0) / args.numbers.length
        const variance = args.numbers.reduce((acc, num) => acc + Math.pow(num - avg, 2), 0) / args.numbers.length
        result = Math.sqrt(variance)
        break
      }
      default:
        throw new Error(`Unsupported operation: ${args.operation}`)
    }
    return result.toString()
  },
})
