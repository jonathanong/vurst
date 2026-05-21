export interface NapiSanitizeRssHtmlOptions {
  proxyImages?: boolean
  imageProxyUrlPrefix?: string
  imageProxySigningKeys?: Array<string>
}

export interface SanitizeRssHtmlResult {
  html: Buffer
  firstImageSrc?: string
}

export interface ExtractDomRemovalsOptions {
  boilerplatePatterns?: Array<string>
}

export interface ExtractDomRemovalsResult {
  cssSelectorsToRemove: Array<string>
  htmlToRemove: Array<string>
}

export interface CrawlerHtmlToMarkdownOptions {
  cssSelectorsToRemove?: Array<string>
  contentSelectors?: Array<string>
  linkTextContentToRemove?: Array<string>
  linkHrefsToRemove?: Array<string>
  linkRelTokensToRemove?: Array<string>
  useTextDensityFilter?: boolean
}

export interface CrawlerHtmlToMarkdownResult {
  title?: string
  meta: Record<string, unknown>
  links: Record<string, unknown>
  content: string
  canonicalUrl?: string
  lang?: string
}

export declare function sanitizeRssHtml(
  html: Buffer,
  options?: NapiSanitizeRssHtmlOptions,
): Promise<SanitizeRssHtmlResult>

export declare function sanitizeRssHtmlBatch(
  inputs: Array<Buffer>,
  options?: NapiSanitizeRssHtmlOptions,
): Promise<Array<SanitizeRssHtmlResult>>

export declare function htmlToEmbeddingText(html: Buffer): Promise<string>

export declare function extractDomRemovals(
  htmlPages: Array<Buffer>,
  options?: ExtractDomRemovalsOptions,
): Promise<ExtractDomRemovalsResult>

export declare function applyDomRemovalsToHtml(
  html: Buffer,
  removals: ExtractDomRemovalsResult,
): Promise<Buffer>

export declare function getContentFromHtml(
  htmlBuffer: Buffer,
  options: CrawlerHtmlToMarkdownOptions,
): Promise<CrawlerHtmlToMarkdownResult>

export declare function sanitizePromptInjection(content: Buffer, isTitle?: boolean): Promise<Buffer>
