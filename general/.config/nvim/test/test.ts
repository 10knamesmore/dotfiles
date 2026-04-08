import { EventEmitter } from 'events'
import type { ReadableStream } from 'stream/web'

// 常量与枚举
const MAX_RETRIES = 3
const API_URL = "https://api.example.com"

enum Status {
  Active = "active",
  Inactive = "inactive",
  Pending = "pending",
}

// 接口与类型
interface Config<T> {
  endpoint: string
  timeout: number
  retries: number
  transform?: (data: T) => T
}

type Result<T> = { ok: true; data: T } | { ok: false; error: string }

// 泛型类
class ApiClient<T extends Record<string, unknown>> extends EventEmitter {
  private config: Config<T>
  private cache: Map<string, T> = new Map()

  constructor(config: Config<T>) {
    super()
    this.config = config
  }

  async fetch(id: string, options?: { force: boolean }): Promise<Result<T>> {
    if (!options?.force && this.cache.has(id)) {
      return { ok: true, data: this.cache.get(id)! }
    }

    for (let i = 0; i < MAX_RETRIES; i++) {
      try {
        const response = await globalThis.fetch(
          `${this.config.endpoint}/${id}`
        )
        const data: T = await response.json()
        const transformed = this.config.transform?.(data) ?? data
        this.cache.set(id, transformed)
        this.emit('fetched', { id, data: transformed })
        return { ok: true, data: transformed }
      } catch (err) {
        if (i === MAX_RETRIES - 1) {
          return { ok: false, error: String(err) }
        }
      }
    }
    return { ok: false, error: "unreachable" }
  }
}

// 使用
const client = new ApiClient<{ name: string; value: number }>({
  endpoint: API_URL,
  timeout: 5000,
  retries: MAX_RETRIES,
  transform: (data) => ({ ...data, name: data.name.toUpperCase() }),
})
