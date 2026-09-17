export type ParentSessionLocationStatus =
  | 'found'
  | 'not_found'
  | 'ambiguous'
  | 'unavailable'

export interface LocateParentSessionResponse {
  status: ParentSessionLocationStatus
  path: string | null
  message: string
}
