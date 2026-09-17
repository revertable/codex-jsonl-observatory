import type { LoadWorkflowState } from '../load-workflow'
import type { TranscriptThemeName } from './transcript-themes'

export function transcriptScopeKey(
  workflow: LoadWorkflowState,
  selectedTheme: TranscriptThemeName,
): string {
  const filter = workflow.filter
  return [
    workflow.loaded_file.metadata?.absolute_path ?? 'unloaded',
    workflow.loaded_file.session?.classification ?? 'unclassified',
    workflow.loaded_file.session?.identity?.thread_id ?? 'no-thread',
    filter.show_you,
    filter.show_codex,
    filter.show_tool_call,
    filter.show_tool_result,
    filter.show_meta,
    selectedTheme,
  ].join('|')
}

export function transcriptScopeChanged(
  previousWorkflow: LoadWorkflowState,
  previousTheme: TranscriptThemeName,
  nextWorkflow: LoadWorkflowState,
  nextTheme: TranscriptThemeName,
): boolean {
  return (
    transcriptScopeKey(previousWorkflow, previousTheme) !==
    transcriptScopeKey(nextWorkflow, nextTheme)
  )
}
