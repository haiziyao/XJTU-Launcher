import DOMPurify from 'dompurify';
import { marked } from 'marked';

marked.use({ gfm: true, breaks: true });

/**
 * Model output is untrusted, so the Markdown is parsed first and then reduced
 * to inert HTML before it reaches `v-html`. Images are dropped: a remote image
 * in a reply would load without the user asking for it.
 */
export function renderMarkdown(text: string): string {
  const html = marked.parse(text, { async: false }) as string;
  return DOMPurify.sanitize(html, {
    FORBID_TAGS: ['style', 'form', 'input', 'textarea', 'select', 'button', 'iframe', 'img'],
    FORBID_ATTR: ['style'],
  });
}
