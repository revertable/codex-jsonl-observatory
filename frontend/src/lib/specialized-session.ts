import type { SessionDescriptorDto } from './parse-contract'

export interface SpecializedSessionPresentation {
  label: string
  title: string
  description: string
  resumeUnavailable: string
  exportUnavailable: string
  parentActionLabel: string | null
}

export function isSpecializedSession(session: SessionDescriptorDto | null): boolean {
  return session !== null && !session.capabilities.can_show_transcript
}

export function presentationForSession(
  session: SessionDescriptorDto,
): SpecializedSessionPresentation {
  switch (session.classification) {
    case 'guardian_review':
      return {
        label: 'Guardian review',
        title: 'Internal review session',
        description:
          'This file records an internal Guardian review session. It is not displayed as an ordinary conversation transcript.',
        resumeUnavailable: 'Not available for Guardian child sessions',
        exportUnavailable: 'Worklog export is not available for Guardian review sessions',
        parentActionLabel: 'Open Parent Session',
      }
    default:
      return {
        label: session.classification,
        title: 'Specialized session',
        description:
          'This session uses a specialized processing path and is not displayed as an ordinary conversation transcript.',
        resumeUnavailable: 'Not available for this session type',
        exportUnavailable: 'Worklog export is not available for this session type',
        parentActionLabel: null,
      }
  }
}
