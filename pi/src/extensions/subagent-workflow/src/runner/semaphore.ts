export class Semaphore {
  private active = 0;
  private readonly queue: Array<() => void> = [];
  private limit: number;

  constructor(capacity: number) {
    if (!Number.isInteger(capacity) || capacity < 1) throw new Error("Semaphore capacity must be positive");
    this.limit = capacity;
  }

  get capacity(): number { return this.limit; }
  get running(): number { return this.active; }
  get pending(): number { return this.queue.length; }

  /**
   * Change the admission ceiling without disturbing active work. Increasing
   * the ceiling admits queued waiters immediately. Decreasing it lets existing
   * holders drain and admits nothing new until running falls below the limit.
   */
  resize(capacity: number): void {
    if (!Number.isInteger(capacity) || capacity < 1) throw new Error("Semaphore capacity must be positive");
    this.limit = capacity;
    this.drain();
  }

  async acquire(signal?: AbortSignal): Promise<() => void> {
    if (signal?.aborted) throw new Error("Semaphore acquire aborted");
    // Reserve the slot synchronously so no acquire() can slip in between a
    // release and an admitted waiter's resume.
    if (this.active >= this.limit) {
      await new Promise<void>((resolve, reject) => {
        const waiter = (): void => {
          signal?.removeEventListener("abort", onAbort);
          resolve();
        };
        const onAbort = (): void => {
          // Still queued: leave without a slot. Already admitted (handed the
          // releaser's slot): resolve won, this reject is a no-op.
          const index = this.queue.indexOf(waiter);
          if (index >= 0) this.queue.splice(index, 1);
          reject(new Error("Semaphore acquire aborted"));
        };
        this.queue.push(waiter);
        signal?.addEventListener("abort", onAbort, { once: true });
      });
    } else {
      this.active += 1;
    }
    let released = false;
    return () => {
      if (released) return;
      released = true;
      this.active -= 1;
      this.drain();
    };
  }

  private drain(): void {
    while (this.active < this.limit) {
      const next = this.queue.shift();
      if (!next) return;
      this.active += 1;
      next();
    }
  }
}
