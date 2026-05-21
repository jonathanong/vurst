export interface Chunk {
  level: number
  header?: string
  headers: Array<string | undefined | null>
  breadcrumb: string
  text: string
  length: number
}

export interface ChunkOptions {
  minLength?: number
  maxLength?: number
  phase?: number
  title?: string
}

export interface NapiMarkdownRenderOptions {
  allowHtml?: boolean
  nofollowLinks?: boolean
  proxyImages?: boolean
  imageProxyUrlPrefix?: string
  imageProxySigningKeys?: Array<string>
}

export interface MarkdownUrls {
  linkUrls: Array<string>
  imageUrls: Array<string>
}

export declare function chunk(text: Buffer, options?: ChunkOptions): Promise<Array<Chunk>>

export declare function renderMarkdownToHtml(
  text: Buffer,
  options?: NapiMarkdownRenderOptions,
): Promise<Buffer>

export declare function renderMarkdownToHtmlBatch(
  inputs: Array<Buffer>,
  options?: NapiMarkdownRenderOptions,
): Promise<Array<Buffer>>

export declare function extractMarkdownUrls(text: Buffer): Promise<MarkdownUrls>
