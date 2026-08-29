export interface MarkdownStreamBuffer {
  push(text: string): string[]
  flush(): string
}

export interface MarkdownStreamBufferOptions {
  maxHoldMs?: number
  now?: () => number
}

export declare function createMarkdownStreamBuffer(
  options?: MarkdownStreamBufferOptions,
): MarkdownStreamBuffer
