export interface SlopDetectionResult {
  flagged: boolean
  confidenceScore: number
  confidenceThreshold: number
  classification: string
  detector: string
  detectorModelVersion: string
}

export declare function detectAiGeneratedText(
  text: Buffer,
  confidenceThreshold?: number,
): Promise<SlopDetectionResult>
