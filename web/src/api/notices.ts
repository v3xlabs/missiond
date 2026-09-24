import { createSignal } from "solid-js";

export type Notice = {
  notice_id: number;
  message: string;
};

const NOTICE_MS = 8000;

// Deliberately application-wide: any write anywhere on the page can fail, and one stack shows them all.
const [notices, setNotices] = createSignal<readonly Notice[]>([]);
const counter = { next: 1 };

export { notices };

export const dismiss = (noticeId: number) => {
  setNotices(current => current.filter(notice => notice.notice_id !== noticeId));
};

/** Every failed write lands here. A refused request the reader cannot see reads as success. */
export const notify = (message: string) => {
  const noticeId = counter.next;

  counter.next += 1;
  setNotices(current => [...current, { notice_id: noticeId, message }]);
  setTimeout(() => dismiss(noticeId), NOTICE_MS);
};
