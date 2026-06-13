// Mock chat messages for browser-only development (no Tauri / no Rust).

import type { ChatMessage } from '$lib/stores/chat.svelte';

function msg(role: ChatMessage['role'], content: string, minutesAgo: number): ChatMessage {
  return {
    id: crypto.randomUUID(),
    role,
    content,
    timestamp: new Date(Date.now() - minutesAgo * 60_000).toISOString()
  };
}

export const MOCK_PATIENT_MESSAGES: ChatMessage[] = [
  msg('assistant', "Hello! I'm here to help you manage your health information. You have 2 new messages from Dr. Patel. Would you like to go through them?", 8),
  msg('user', "Yes, let's start with the questionnaire.", 7),
  msg('assistant', "Great. Dr. Patel sent a pre-visit questionnaire. First question: On a scale of 1–10, how would you rate your pain level over the past week?", 6),
  msg('user', "About a 4. It's manageable but noticeable in the mornings.", 5),
  msg('assistant', "Noted, I've recorded that as a 4 — manageable but noticeable in the mornings. Next question: Have you been taking your blood pressure medication consistently?", 4),
  msg('user', "Yes, every morning with breakfast.", 3),
  msg('assistant', "Recorded. Last question: Any new symptoms since your last visit — dizziness, shortness of breath, or swelling in your legs?", 2),
  msg('user', "No new symptoms.", 1),
  msg('assistant', "All done. I've saved your responses to the questionnaire. Is there anything else you'd like to review before your appointment?", 0)
];

export const MOCK_PROVIDER_MESSAGES: ChatMessage[] = [
  msg('assistant', "Good day. I'm ready to assist with this patient visit. Patient data is filtered according to their privacy settings. How can I help?", 10),
  msg('user', "Give me a quick summary of the patient's recent questionnaire responses.", 9),
  msg('assistant', "Maria Gonzalez completed the pre-visit questionnaire today. Key responses: pain level 4/10 (manageable, worse in mornings), blood pressure medication taken consistently, no new symptoms since last visit — no dizziness, shortness of breath, or leg swelling.", 8),
  msg('user', "Any previous visit transcripts I should review?", 7),
  msg('assistant', "There is one meeting transcript from the April 14th visit. It covers a discussion of the medication adjustment and a follow-up plan for blood pressure monitoring. Would you like me to summarize it?", 6),
  msg('user', "Yes please.", 5),
  msg('assistant', "April 14th summary: Patient reported improved readings after dose adjustment. Agreed to monitor twice daily for 4 weeks and report outliers. Next steps were a lab panel (ordered) and a follow-up in 8 weeks — which aligns with today's visit.", 3)
];
