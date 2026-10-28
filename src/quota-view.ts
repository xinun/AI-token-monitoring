// Keep quota elements in place so fresh data can animate without rebuilding cards.
function syncNode(current: Node, next: Node): void {
  if (current.nodeType !== next.nodeType || current.nodeName !== next.nodeName ||
      current instanceof Element && next instanceof Element &&
      (current.getAttribute('data-provider') !== next.getAttribute('data-provider') ||
       current.getAttribute('data-quota-key') !== next.getAttribute('data-quota-key'))) {
    current.parentNode?.replaceChild(next.cloneNode(true), current);
    return;
  }
  if (current instanceof Element && next instanceof Element) {
    for (const attribute of [...current.attributes]) {
      if (!next.hasAttribute(attribute.name)) current.removeAttribute(attribute.name);
    }
    for (const attribute of [...next.attributes]) {
      if (current.getAttribute(attribute.name) !== attribute.value) current.setAttribute(attribute.name, attribute.value);
    }
    syncChildren(current, next);
  } else if (current.nodeValue !== next.nodeValue) current.nodeValue = next.nodeValue;
}

function syncChildren(current: Node, next: Node): void {
  const children = [...current.childNodes];
  const nextChildren = [...next.childNodes];
  for (let index = 0; index < Math.max(children.length, nextChildren.length); index++) {
    if (!nextChildren[index]) children[index].parentNode?.removeChild(children[index]);
    else if (!children[index]) current.appendChild(nextChildren[index].cloneNode(true));
    else syncNode(children[index], nextChildren[index]);
  }
}

function amount(fill: HTMLElement): number | null {
  const value = fill.dataset.remaining;
  if (value === undefined || value === '') return null;
  const number = Number(value);
  return Number.isFinite(number) && number >= 0 && number <= 100 ? number : null;
}

export function stopViewAnimations(root: HTMLElement): void {
  root.getAnimations({ subtree: true }).forEach(animation => animation.cancel());
}

export function patchProviderCards(container: HTMLElement, html: string, allowMotion: boolean): void {
  const template = document.createElement('template');
  template.innerHTML = html;
  const previous = new Map([...container.querySelectorAll<HTMLElement>('[data-gauge-key]')]
    .map(fill => [fill.dataset.gaugeKey!, fill]));
  const motions: { fill: HTMLElement; from: string; to: string }[] = [];
  for (const next of template.content.querySelectorAll<HTMLElement>('[data-gauge-key]')) {
    const current = previous.get(next.dataset.gaugeKey!);
    if (!current) continue;
    const before = amount(current);
    const after = amount(next);
    if (!allowMotion || before === null || after === null) {
      current.getAnimations().forEach(animation => animation.cancel());
    } else if (before !== after) {
      const running = current.getAnimations();
      const from = running.length ? getComputedStyle(current).transform : current.style.transform;
      running.forEach(animation => animation.cancel());
      motions.push({ fill: current, from, to: next.style.transform });
    }
  }
  syncChildren(container, template.content);
  for (const { fill, from, to } of motions) {
    if (container.contains(fill)) fill.animate([{ transform: from }, { transform: to }], {
      duration: 350,
      easing: 'cubic-bezier(.2,.7,.2,1)',
    });
  }
}
