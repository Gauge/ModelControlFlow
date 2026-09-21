import re


def _escape(text):
    return text.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')


def _inline(text):
    text = text.strip()
    pieces = re.split(r'(`[^`]*`)', text)
    out = []
    for piece in pieces:
        if len(piece) >= 2 and piece.startswith('`') and piece.endswith('`'):
            out.append('<code>' + _escape(piece[1:-1]) + '</code>')
        else:
            out.append(_format(_escape(piece)))
    return ''.join(out)


def _format(text):
    text = re.sub(r'\[([^\]]*)\]\(([^)]*)\)', lambda m: '<a href="' + m.group(2) + '">' + _format(m.group(1)) + '</a>', text)
    text = re.sub(r'\*\*(.+?)\*\*', r'<strong>\1</strong>', text)
    text = re.sub(r'\*(.+?)\*', r'<em>\1</em>', text)
    return text


def render(markdown):
    lines = markdown.split('\n')
    blocks = []
    paragraph = []
    items = []
    kind = None

    def end_paragraph():
        if paragraph:
            blocks.append('<p>' + ' '.join(_inline(line) for line in paragraph) + '</p>')
            paragraph.clear()

    def end_list():
        nonlocal kind
        if items:
            blocks.append(f'<{kind}>' + ''.join(f'<li>{_inline(item)}</li>' for item in items) + f'</{kind}>')
            items.clear()
        kind = None

    at = 0
    while at < len(lines):
        line = lines[at]
        at += 1
        if line == '```':
            end_paragraph()
            end_list()
            code = []
            while at < len(lines) and lines[at] != '```':
                code.append(lines[at])
                at += 1
            at += 1
            blocks.append('<pre><code>' + _escape('\n'.join(code)) + '</code></pre>')
            continue
        if line.strip() == '':
            end_paragraph()
            end_list()
            continue
        heading = re.match(r'(#{1,6}) (.*)$', line)
        if heading:
            end_paragraph()
            end_list()
            level = len(heading.group(1))
            blocks.append(f'<h{level}>{_inline(heading.group(2))}</h{level}>')
            continue
        bullet = re.match(r'- (.*)$', line)
        numbered = re.match(r'\d+\. (.*)$', line)
        if bullet or numbered:
            end_paragraph()
            wanted = 'ul' if bullet else 'ol'
            if kind != wanted:
                end_list()
                kind = wanted
            items.append((bullet or numbered).group(1))
            continue
        end_list()
        paragraph.append(line)
    end_paragraph()
    end_list()
    return '\n'.join(blocks)
