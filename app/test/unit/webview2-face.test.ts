import { describe, expect, it } from 'vitest';
import { decodeText, resourceOf, webView2Message } from '../../src/engine/webview2-log.ts';
import { dirOf, faceClipPath, faceRect, joinPath, rewriteRefs, splitStyles, subtractBoxes, withoutCspBlocked, withoutScripts } from '../../src/ui/webview2-surface.ts';

describe('the WebView2 model lines the page needs (Issue #1362)', () => {
  it('reads the controller bounds with the window origin', () => {
    expect(webView2Message('webview2: put_Bounds (ICoreWebView2Controller) bounds=0,0,365,299 origin=-5,139')).toEqual({ t: 'webview2', op: 'bounds', left: 0, top: 0, right: 365, bottom: 299, originX: -5, originY: 139 });
    expect(webView2Message('webview2: put_Bounds (ICoreWebView2Controller) bounds=0,0,364,905 origin=none')).toEqual({ t: 'webview2', op: 'bounds', left: 0, top: 0, right: 364, bottom: 905, originX: null, originY: null });
  });

  it('reads the visibility and the Navigate of the bundled page', () => {
    expect(webView2Message('[err] webview2: put_IsVisible (ICoreWebView2Controller) isVisible=0')).toEqual({ t: 'webview2', op: 'visible', visible: false });
    const line = 'webview2: Navigate (ICoreWebView2) uri="file://C:\\\\Program Files\\\\Image-Line\\\\FL Studio 21\\\\System\\\\Internet\\\\error-offline.html" units=82';
    expect(webView2Message(line)).toEqual({ t: 'webview2', op: 'navigate', uri: 'file://C:\\Program Files\\Image-Line\\FL Studio 21\\System\\Internet\\error-offline.html', resource: 'resources/System/Internet/error-offline.html' });
  });

  it('skips the fire lines, the other methods, and a text that the model cut', () => {
    expect(webView2Message('webview2: fire NavigationStarting (ICoreWebView2) uri="about:blank"')).toBeNull();
    expect(webView2Message('webview2: get_Source (ICoreWebView2) -> "about:blank"')).toBeNull();
    expect(webView2Message('webview2: Navigate (ICoreWebView2) uri="file://…" units=400')).toBeNull();
    expect(webView2Message('fle: something else')).toBeNull();
  });

  it('decodes the units of a text (the backslash doubled, the others as \\uXXXX)', () => {
    expect(decodeText('a\\\\b\\u0022c\\u00e9')).toBe('a\\b"cé');
    expect(resourceOf('https://example.invalid/x')).toBeNull();
  });

  it('reads a posted message and an executed script', () => {
    expect(webView2Message('webview2: PostWebMessageAsJson (ICoreWebView2) message="\\u007b\\u0022a\\u0022:1\\u007d" units=7')).toEqual({ t: 'webview2', op: 'post', json: '{"a":1}' });
    expect(webView2Message('webview2: ExecuteScript (ICoreWebView2) script="1+1" units=3 handler=0x7f00')).toEqual({ t: 'webview2', op: 'exec', script: '1+1' });
  });
});

describe('the face of the Browser panel (Issue #1362)', () => {
  it('places the face at the window origin plus the bounds, in CSS pixels', () => {
    expect(faceRect({ left: 0, top: 0, right: 364, bottom: 905 }, { x: -5, y: 139 }, 1)).toEqual({ left: -5, top: 139, width: 364, height: 905 });
    expect(faceRect({ left: 0, top: 0, right: 365, bottom: 329 }, null, 0.5)).toEqual({ left: 0, top: 0, width: 182.5, height: 164.5 });
  });

  it('takes the style blocks out of the markup (the page makes them style sheets: the CSP refuses inline styles)', () => {
    const html = '<!DOCTYPE html>\n<html>\n<head>\n<style type="text/css">\n  body { color: red; }\n</style>\n<style>p{}</style>\n</head>\n<body><p>b</p></body></html>';
    const { markup, styles } = splitStyles(html);
    expect(styles).toEqual(['\n  body { color: red; }\n', 'p{}']);
    expect(markup).not.toContain('<style');
    expect(markup).toContain('<p>b</p>');
  });

  it('takes the scripts and the event handlers out of the markup (Issue #1419: a frame without allow-scripts logs an error for each, on every load)', () => {
    const html = '<body onload="f()"><p onclick="g()">b</p><script>f()</script><script src="x.js"></script><img src="a.png" alt="x" onerror=\'h()\'></body>';
    expect(withoutScripts(html)).toBe('<body><p>b</p><img src="a.png" alt="x"></body>');
    expect(withoutScripts('<p>no script</p>')).toBe('<p>no script</p>');
    expect(withoutScripts('<Script language="Javascript">\ndocument.body.scroll="no"\n</script><form onSubmit="return jumpit()"><input type="text" value="http://"></form>')).toBe('<form><input type="text" value="http://"></form>');
  });

  it('takes the inline style attributes and the data: fonts out (Issue #1419: the page CSP stops each one, and logs an error on every load)', () => {
    const html = '<svg version="1.1" style="enable-background:new 0 0 1 1;"><g>x</g></svg><style>@font-face {\n  font-family: \'gilroy\';\n  src: url(data:font/truetype;base64,AAAA) format(\'truetype\');\n}\np { color: red; }</style>';
    expect(withoutCspBlocked(html)).toBe('<svg version="1.1"><g>x</g></svg><style>\np { color: red; }</style>');
    expect(withoutCspBlocked('<p class="a">b</p>')).toBe('<p class="a">b</p>');
  });

  it('joins a reference to the directory of the document', () => {
    expect(joinPath('System/Internet/', 'img/a.png')).toBe('System/Internet/img/a.png');
    expect(joinPath('System/Internet/', '../Art/b.svg')).toBe('System/Art/b.svg');
    expect(dirOf('resources/System/Internet/error-offline.html')).toBe('System/Internet/');
  });

  it('rewrites the relative references that name a bundled resource, and leaves the others', async () => {
    const files = new Map([['resources/System/Internet/img/a.png', new Uint8Array([1, 2])]]);
    const html = '<img src="img/a.png"><a href="#offlineData">x</a><img src="https://example.invalid/z.png"><div style="background:url(img/a.png)"></div><img src="img/missing.png">';
    const out = await rewriteRefs(html, 'System/Internet/', async (p) => files.get(p) ?? null, (b, p) => `blob:test/${p}:${b.length}`);
    expect(out).toContain('src="blob:test/System/Internet/img/a.png:2"');
    expect(out).toContain('url(blob:test/System/Internet/img/a.png:2)');
    expect(out).toContain('href="#offlineData"');
    expect(out).toContain('src="https://example.invalid/z.png"');
    expect(out).toContain('src="img/missing.png"');
  });
});

describe('the part of the face that the windows over it do not cover (Issue #1376)', () => {
  const face = { left: 0, top: 0, right: 100, bottom: 80 };
  const area = (boxes: { left: number; top: number; right: number; bottom: number }[]): number => boxes.reduce((s, b) => s + (b.right - b.left) * (b.bottom - b.top), 0);

  it('is the whole face when no window is over it', () => {
    expect(subtractBoxes(face, [])).toEqual([face]);
    expect(faceClipPath(face, [])).toBe('');
  });

  it('is the face minus a window that is in the middle: disjoint parts with the area of the rest', () => {
    const over = [{ left: 30, top: 20, right: 60, bottom: 50 }];
    const parts = subtractBoxes(face, over);
    expect(area(parts)).toBe(100 * 80 - 30 * 30);
    for (const p of parts) expect(p.right > p.left && p.bottom > p.top).toBe(true);
    expect(faceClipPath(face, over)).toBe('path(evenodd, "M0 0H100V20H0Z M0 50H100V80H0Z M0 20H30V50H0Z M60 20H100V50H60Z")');
  });

  it('subtracts overlapping windows once each, and a window that does not meet the face changes nothing', () => {
    const over = [{ left: -10, top: -10, right: 50, bottom: 40 }, { left: 40, top: 30, right: 120, bottom: 100 }, { left: 200, top: 0, right: 300, bottom: 80 }];
    const parts = subtractBoxes(face, over);
    const covered = (x: number, y: number): boolean => over.some((o) => x >= o.left && x < o.right && y >= o.top && y < o.bottom);
    for (let x = 0; x < 100; x += 1) for (let y = 0; y < 80; y += 1) {
      const inside = parts.filter((p) => x >= p.left && x < p.right && y >= p.top && y < p.bottom).length;
      expect(inside).toBe(covered(x + 0.5, y + 0.5) ? 0 : 1);
    }
  });

  it('is empty when a window covers the whole face', () => {
    expect(subtractBoxes(face, [{ left: -5, top: -5, right: 200, bottom: 200 }])).toEqual([]);
    expect(faceClipPath(face, [{ left: -5, top: -5, right: 200, bottom: 200 }])).toBe('path(evenodd, "")');
  });
});
