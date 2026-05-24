export interface SanitizePromptInjectionOptions {
  isTitle?: boolean
}

export interface WrapExternalContentOptions {
  source: string
  contentType?: string
  includeReminder?: boolean
}

export declare function sanitizePromptInjection(
  content: string,
  options?: SanitizePromptInjectionOptions,
): Promise<string>

export declare function sanitizeRssContent(html: string): Promise<string>

export declare function wrapExternalContent(
  content: string,
  options: WrapExternalContentOptions,
): string
