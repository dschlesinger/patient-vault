import type { DecryptedPayload } from '$lib/tauri/commands';

export type SessionTab = 'chat' | 'messages' | 'questionnaires' | 'documents' | 'providers';

/** Tabs for the provider session (no provider-management tab). */
export const SESSION_TABS: [SessionTab, string][] = [
	['chat', 'Chat'],
	['messages', 'Messages'],
	['questionnaires', 'Questionnaires'],
	['documents', 'Documents']
];

/** Tabs for the patient session — includes provider connection/management. */
export const PATIENT_SESSION_TABS: [SessionTab, string][] = [
	...SESSION_TABS,
	['providers', 'Providers']
];

export interface DocumentMeta {
	name: string;
	type: string;
	path: string;
	size?: number;
	provider_id?: string;
}

export function messagePayloads(payloads: DecryptedPayload[]): DecryptedPayload[] {
	return payloads.filter((p) => p.type === 'message');
}

export function questionnairePayloads(payloads: DecryptedPayload[]): DecryptedPayload[] {
	return payloads.filter((p) => p.type === 'questionnaire');
}

export function documentPayloads(payloads: DecryptedPayload[]): DecryptedPayload[] {
	return payloads.filter((p) => p.type === 'document');
}

export function parseMessageContent(content: string): string {
	try {
		const parsed = JSON.parse(content) as { content?: string };
		if (parsed.content) return parsed.content;
		return content;
	} catch {
		return content;
	}
}

export function parseQuestionnaireQuestions(content: string): string[] {
	try {
		const parsed = JSON.parse(content) as { questions?: string[] };
		if (Array.isArray(parsed.questions) && parsed.questions.length > 0) {
			return parsed.questions;
		}
	} catch {
		// fall through
	}
	return [content];
}

export function parseDocumentMeta(content: string): DocumentMeta | null {
	try {
		return JSON.parse(content) as DocumentMeta;
	} catch {
		return null;
	}
}

export function payloadLabel(type: DecryptedPayload['type']): string {
	if (type === 'questionnaire') return 'Questionnaire';
	if (type === 'message') return 'Message';
	return 'Document';
}

export function formatFileSize(bytes?: number): string {
	if (!bytes) return '';
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
	return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function fileTypeLabel(mime?: string): string {
	if (!mime) return 'File';
	if (mime.includes('pdf')) return 'PDF';
	if (mime.includes('image')) return 'Image';
	if (mime.includes('word') || mime.includes('document')) return 'Word';
	return mime.split('/').pop()?.toUpperCase() ?? 'File';
}

/**
 * Human-readable label for the provider a payload came from, using the patient's
 * assigned `display_name`. Falls back gracefully for legacy links (empty
 * `provider_id`) or providers no longer in the local link list.
 */
export function providerLabel(
	providerId: string,
	links: { provider_id: string; display_name?: string }[] = []
): string {
	const match = links.find((l) => l.provider_id === providerId);
	if (match?.display_name) return match.display_name;

	// Legacy single-provider link stored before display_name existed.
	if (!providerId && links.length === 1 && links[0].display_name) {
		return links[0].display_name;
	}
	if (!providerId) return 'Your provider';
	return 'Unknown provider';
}

export function formatRegisteredAt(value: string): string {
	const unix = Number(value);
	if (!Number.isNaN(unix) && unix > 1e9) {
		return new Date(unix * 1000).toLocaleDateString();
	}
	const parsed = new Date(value);
	return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleDateString();
}
