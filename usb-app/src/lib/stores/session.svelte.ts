// Active session type for the USB app.
// 'none' = startup/home screen, 'patient' = patient session, 'provider' = provider session.

export type SessionType = 'none' | 'patient' | 'provider';

let current = $state<SessionType>('none');

export const session = {
  get current() {
    return current;
  },
  set(type: SessionType) {
    current = type;
  }
};
