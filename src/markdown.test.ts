// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { renderMarkdown } from './markdown';

describe('renderMarkdown', () => {
  it('renders the formatting a model usually writes', () => {
    const html = renderMarkdown('**加粗**\n\n- 一项\n- 另一项\n\n`code`\n\n```\nblock\n```');
    expect(html).toContain('<strong>加粗</strong>');
    expect(html).toContain('<li>一项</li>');
    expect(html).toContain('<code>code</code>');
    expect(html).toContain('<pre><code>block\n</code></pre>');
  });

  it('keeps single newlines readable', () => {
    expect(renderMarkdown('第一行\n第二行')).toContain('第一行<br>第二行');
  });

  it('strips active content instead of trusting the model', () => {
    const html = renderMarkdown(
      '<script>alert(1)</script><img src="https://example.com/x.png" onerror="alert(1)"><style>body{}</style>',
    );
    expect(html).not.toContain('<script');
    expect(html).not.toContain('<img');
    expect(html).not.toContain('<style');
  });

  it('drops unsafe link targets', () => {
    const html = renderMarkdown('[点我](javascript:alert(1))');
    expect(html).not.toContain('javascript:');
  });

  it('keeps ordinary links for the page to open externally', () => {
    expect(renderMarkdown('[成绩](https://jwc.xjtu.edu.cn)')).toContain(
      'href="https://jwc.xjtu.edu.cn"',
    );
  });
});
