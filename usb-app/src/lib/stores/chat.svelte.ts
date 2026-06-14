// Chat state for the LLM conversational interface.

export type MessageRole = 'assistant' | 'user';

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  /** ISO timestamp */
  timestamp: string;
}

let messages = $state<ChatMessage[]>([]);
let streamingTranscript = $state('');
let isLlmThinking = $state(false);
let isMicActive = $state(false);

export const chat = {
  get messages() {
    return messages;
  },
  get streamingTranscript() {
    return streamingTranscript;
  },
  get isLlmThinking() {
    return isLlmThinking;
  },
  get isMicActive() {
    return isMicActive;
  },
  addMessage(role: MessageRole, content: string) {
    messages.push({
      id: crypto.randomUUID(),
      role,
      content,
      timestamp: new Date().toISOString()
    });
  },
  setStreamingTranscript(text: string) {
    streamingTranscript = text;
  },
  setLlmThinking(val: boolean) {
    isLlmThinking = val;
  },
  setMicActive(val: boolean) {
    isMicActive = val;
  },
  clear() {
    messages = [];
    streamingTranscript = '';
    isLlmThinking = false;
    isMicActive = false;
  }
};
