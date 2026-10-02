/**
 * Inspect one resolved element; Rust owns locating, polling, deadlines and input dispatch.
 * @this {HTMLElement}
 * @param {'text'|'visible'|'click'|'fill'|'focus'|'files'} action
 * @returns {{connected: boolean, visible?: boolean, ready?: boolean, x?: number, y?: number, text?: string, multiple?: boolean}}
 */
function(action) {
  const element = this;
  if (!element.isConnected) return { connected: false };
  if (action === 'text') return { connected: true, text: element.innerText ?? element.textContent ?? '' };
  if (action === 'files') {
    if (!(element instanceof HTMLInputElement) || element.type !== 'file') throw new Error('set_input_files requires an input[type=file]');
    return { connected: true, ready: true, multiple: element.multiple };
  }
  const style = getComputedStyle(element);
  const rect = element.getBoundingClientRect();
  const visible = style.visibility !== 'hidden' && style.visibility !== 'collapse' && rect.width > 0 && rect.height > 0;
  if (action === 'visible' || !visible) return { connected: true, visible, ready: false };
  if (element.matches(':disabled') || element.getAttribute('aria-disabled') === 'true') return { connected: true, visible, ready: false };
  element.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' });
  if (action === 'focus') {
    element.focus();
    return { connected: true, visible, ready: document.activeElement === element };
  }
  if (action === 'fill') {
    if (element.readOnly) return { connected: true, visible, ready: false };
    const textInput = element instanceof HTMLInputElement && ['text', 'search', 'email', 'url', 'tel', 'password'].includes(element.type);
    if (textInput || element instanceof HTMLTextAreaElement) {
      element.focus();
      element.select();
    } else if (element.isContentEditable) {
      element.focus();
      const range = document.createRange();
      range.selectNodeContents(element);
      const selection = window.getSelection();
      selection.removeAllRanges();
      selection.addRange(range);
    } else {
      throw new Error('fill requires a text input, textarea or contenteditable element');
    }
    return { connected: true, visible, ready: true };
  }
  const bounds = element.getBoundingClientRect();
  const left = Math.max(0, bounds.left), right = Math.min(innerWidth, bounds.right);
  const top = Math.max(0, bounds.top), bottom = Math.min(innerHeight, bounds.bottom);
  if (right <= left || bottom <= top) return { connected: true, visible, ready: false };
  const x = (left + right) / 2, y = (top + bottom) / 2;
  const hit = document.elementFromPoint(x, y);
  return { connected: true, visible, ready: hit === element || element.contains(hit), x, y };
}
