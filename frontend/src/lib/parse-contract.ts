export type EntryKind =
  | 'context'
  | 'task'
  | 'you'
  | 'codex'
  | 'tool_call'
  | 'tool_result'
  | 'system'

export interface ParseRequestDto {
  path: string
  filter: FilterDto
}

export interface FilterDto {
  show_you: boolean
  show_codex: boolean
  show_tool_call: boolean
  show_tool_result: boolean
  show_meta: boolean
}

export interface ParseResponseDto {
  source: LoadedFileMetadataDto
  session: SessionDescriptorDto
  parsed_chat_log: ParsedChatLogDto
}

export interface SessionDescriptorDto {
  classification: string
  identity: SessionIdentityDto | null
  capabilities: SessionCapabilitiesDto
}

export interface SessionIdentityDto {
  thread_id: string | null
  session_id: string | null
  parent_thread_id: string | null
  originator: string | null
  thread_source: string | null
  source: SessionSourceDto
  history_mode: string | null
  subagent_history_start_ordinal: number | null
}

export interface SessionSourceDto {
  kind: string
  value: string | null
}

export interface SessionCapabilitiesDto {
  can_show_transcript: boolean
  can_resume: boolean
  can_export_worklog: boolean
  can_open_parent_session: boolean
}

export interface LoadedFileMetadataDto {
  file_name: string | null
  absolute_path: string
  session_id: string | null
  resume_command: string | null
}

export interface ParsedChatLogDto {
  entries: RenderedEntryDto[]
  transcript_blocks: TranscriptBlockDto[]
  counters: ParseCountersDto
  observed_event_counts: ObservedEventCountDto[]
}

export interface RenderedEntryDto {
  kind: EntryKind
  label: string
  content: string
}

export interface TranscriptBlockDto {
  entry_type: EntryKind
  label: string
  title: string
  content: string
}

export interface ParseCountersDto {
  parsed_candidates: number
  total_entries: number
  visible_entries: number
  ignored_lines: number
  malformed_lines: number
}

export interface ObservedEventCountDto {
  event: string
  count: number
}

export interface ErrorResponseDto {
  error: ApiErrorDto
}

export interface ApiErrorDto {
  code: string
  message: string
}
