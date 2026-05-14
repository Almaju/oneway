// Oneway syntax highlighting for mdBook.
//
// mdBook ships a small highlight.js bundle. This file registers an extra
// `oneway` language on top of it and re-highlights any code block tagged
// with ```oneway` (or ```ow`).

(function () {
    function defineOneway(hljs) {
        return {
            name: 'Oneway',
            aliases: ['ow'],
            keywords: {
                keyword: 'match mut use Self impl extern while for',
                type:
                    'Bit Bool Byte Bytes Clock Empty Filesystem Float Hex ' +
                    'Int List Map Network Noop Option Ord Path Random Result ' +
                    'Self Stderr Stdin Stdout String',
                literal:
                    'False True Off On None Some Ok Err ' +
                    'Equal Greater Less Noop',
                built_in: 'Rust'
            },
            contains: [
                {
                    className: 'string',
                    begin: '"',
                    end: '"',
                    contains: [{ begin: '\\\\.' }]
                },
                {
                    className: 'number',
                    variants: [
                        { begin: '\\b0x[a-fA-F0-9_]+\\b' },
                        { begin: '\\b\\d+\\.\\d+\\b' },
                        { begin: '\\b\\d+\\b' }
                    ]
                },
                {
                    // Trait/type identifiers (PascalCase)
                    className: 'type',
                    begin: '\\b[A-Z][A-Za-z0-9_]*\\b'
                },
                {
                    // Private-method sigil: `*helper`
                    className: 'symbol',
                    begin: '\\*[a-z][A-Za-z0-9_]*'
                },
                {
                    // Arrows and propagation
                    className: 'operator',
                    begin: '(->|=>|\\?|\\.\\.\\.)'
                }
            ]
        };
    }

    function highlightOnewayBlocks() {
        var blocks = document.querySelectorAll(
            'pre code.language-oneway, pre code.language-ow'
        );
        var highlightFn = hljs.highlightElement || hljs.highlightBlock;
        blocks.forEach(function (block) {
            // mdBook's loader may have already tagged the block as plain
            // text. Reset state, then re-run highlighting under `oneway`.
            block.removeAttribute('data-highlighted');
            block.classList.remove('hljs');
            var raw = block.textContent;
            block.textContent = raw;
            block.classList.add('language-oneway');
            highlightFn.call(hljs, block);
        });
    }

    function init() {
        if (typeof hljs === 'undefined') {
            // highlight.js not loaded yet — try again shortly.
            setTimeout(init, 50);
            return;
        }
        if (!hljs.getLanguage('oneway')) {
            hljs.registerLanguage('oneway', defineOneway);
        }
        highlightOnewayBlocks();
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
