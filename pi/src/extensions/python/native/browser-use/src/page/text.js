/**
 * Match normalized text content, choosing the deepest matching elements.
 * @param {string} text
 * @param {boolean} exact Exact matching is case-sensitive; substring matching is case-insensitive.
 * @returns {Element[]}
 */
(text, exact) => {
  const normalize = value => value.replace(/\s+/g, ' ').trim();
  const expected = normalize(text);
  const eligible = element => !['SCRIPT', 'STYLE', 'NOSCRIPT', 'HEAD'].includes(element.tagName);
  const cache = new Map();
  const contents = element => {
    if (!eligible(element)) return '';
    if (cache.has(element)) return cache.get(element);
    let text = '';
    if (element instanceof HTMLInputElement && ['button', 'submit', 'reset'].includes(element.type)) {
      text = element.value;
    } else {
      for (const child of element.childNodes) {
        if (child.nodeType === Node.TEXT_NODE) text += child.textContent;
        else if (child.nodeType === Node.ELEMENT_NODE) text += contents(child);
      }
    }
    cache.set(element, text);
    return text;
  };
  const matches = element => {
    if (!eligible(element)) return false;
    const actual = normalize(contents(element));
    return exact ? actual === expected : actual.toLowerCase().includes(expected.toLowerCase());
  };
  return Array.from(document.querySelectorAll('body, body *')).filter(element =>
    matches(element) && !Array.from(element.children).some(matches)
  );
}
