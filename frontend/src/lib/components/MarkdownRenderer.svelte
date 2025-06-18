<script>
	import { marked } from 'marked';
	import hljs from 'highlight.js';
	import { onMount, onDestroy, tick } from 'svelte';

	export let content = '';
	
	let renderedHtml = '';
	let containerRef;
	let copyButtons = new Map();
	let intersectionObserver;
	let copiedStates = new Map();
	let codeBlockIndex = 0;

	// Configure marked with proper renderer
	function setupMarked() {
		const renderer = new marked.Renderer();

		renderer.code = function(code, language) {
			const blockId = `code-block-${codeBlockIndex++}`;
			
			
			// Handle the case where code might be an object and extract language
			let codeString;
			let actualLanguage = language;
			
			if (typeof code === 'object' && code !== null) {
				// Extract the actual code content
				if (code.text) {
					codeString = code.text;
				} else if (code.raw) {
					codeString = code.raw;
				} else if (code.content) {
					codeString = code.content;
				} else {
					codeString = JSON.stringify(code);
				}
				
				// Extract language if not provided as parameter
				if (!actualLanguage && code.lang) {
					actualLanguage = code.lang;
				}
			} else {
				codeString = String(code || '');
			}
			
			let highlightedCode;
			let detectedLanguage = actualLanguage;
			
			
			if (actualLanguage && hljs.getLanguage(actualLanguage)) {
				try {
					const result = hljs.highlight(codeString, { language: actualLanguage });
					highlightedCode = result.value;
					detectedLanguage = actualLanguage;
				} catch (err) {
					console.warn('Failed to highlight code with language:', actualLanguage, err);
					highlightedCode = codeString;
				}
			} else {
				try {
					const result = hljs.highlightAuto(codeString);
					highlightedCode = result.value;
					detectedLanguage = result.language || actualLanguage || 'text';
				} catch (err) {
					console.warn('Failed to auto-highlight code:', err);
					highlightedCode = codeString;
					detectedLanguage = actualLanguage || 'text';
				}
			}
			
			const languageClass = detectedLanguage ? `language-${detectedLanguage}` : '';
			const languageLabel = detectedLanguage || 'text';
			
			return `
				<div class="code-block-wrapper" data-block-id="${blockId}">
					<div class="code-block-header">
						<span class="code-language">${languageLabel}</span>
					</div>
					<div class="code-block-container">
						<pre><code class="hljs ${languageClass}">${highlightedCode}</code></pre>
						<button class="copy-button" data-code="${encodeURIComponent(codeString)}" data-block-id="${blockId}">
							<span class="copy-icon">
								<svg width="16" height="16" fill="currentColor" viewBox="0 0 24 24">
									<path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
								</svg>
							</span>
							<span class="check-icon" style="display: none;">
								<svg width="16" height="16" fill="currentColor" viewBox="0 0 24 24">
									<path d="M9 16.2L4.8 12l-1.4 1.4L9 19 21 7l-1.4-1.4L9 16.2z"/>
								</svg>
							</span>
						</button>
					</div>
				</div>
			`;
		};

		marked.setOptions({
			renderer: renderer,
			breaks: true,
			gfm: true
		});
	}

	// Initialize marked on component creation
	setupMarked();

	$: {
		if (content) {
			codeBlockIndex = 0; // Reset counter for each render
			try {
				renderedHtml = marked.parse(content);
			} catch (error) {
				console.error('Failed to parse markdown:', error);
				renderedHtml = `<p>${content}</p>`; // Fallback to plain text wrapped in p tag
			}
		} else {
			renderedHtml = '';
		}
	}

	function setupCopyButtons() {
		if (!containerRef) return;

		const buttons = containerRef.querySelectorAll('.copy-button');
		
		buttons.forEach(button => {
			const blockId = button.dataset.blockId;
			const encodedCode = button.dataset.code;
			
			if (!encodedCode) return;
			
			const code = decodeURIComponent(encodedCode);
			
			// Remove existing listener if it exists
			const existingHandler = copyButtons.get(blockId);
			if (existingHandler) {
				button.removeEventListener('click', existingHandler);
			}
			
			// Create new handler
			const handler = async () => {
				try {
					await navigator.clipboard.writeText(code);
					showCopiedState(button, blockId);
				} catch (err) {
					console.error('Failed to copy code:', err);
				}
			};
			
			// Store handler and add listener
			copyButtons.set(blockId, handler);
			button.addEventListener('click', handler);
		});

		setupViewportTracking();
	}

	function showCopiedState(button, blockId) {
		const copyIcon = button.querySelector('.copy-icon');
		const checkIcon = button.querySelector('.check-icon');
		
		if (copyIcon && checkIcon) {
			copyIcon.style.display = 'none';
			checkIcon.style.display = 'inline-flex';
			button.classList.add('copied');
			
			// Clear any existing timeout
			if (copiedStates.has(blockId)) {
				clearTimeout(copiedStates.get(blockId));
			}
			
			// Reset after 2 seconds
			const timeout = setTimeout(() => {
				copyIcon.style.display = 'inline-flex';
				checkIcon.style.display = 'none';
				button.classList.remove('copied');
				copiedStates.delete(blockId);
			}, 2000);
			
			copiedStates.set(blockId, timeout);
		}
	}

	function setupViewportTracking() {
		if (!containerRef || intersectionObserver) return;

		intersectionObserver = new IntersectionObserver((entries) => {
			entries.forEach(entry => {
				const button = entry.target.querySelector('.copy-button');
				if (button) {
					if (entry.isIntersecting) {
						button.classList.add('visible');
					} else {
						button.classList.remove('visible');
					}
				}
			});
		}, {
			threshold: 0.1,
			rootMargin: '10px'
		});

		const codeBlocks = containerRef.querySelectorAll('.code-block-wrapper');
		codeBlocks.forEach(block => {
			intersectionObserver.observe(block);
		});

		// Add scroll listener for dynamic positioning
		// Listen to both window and document scroll events
		window.addEventListener('scroll', updateButtonPositions, { passive: true });
		document.addEventListener('scroll', updateButtonPositions, { passive: true });
		window.addEventListener('resize', updateButtonPositions, { passive: true });
		
		// Also find the scrolling container and listen to it
		let scrollContainer = containerRef.closest('.chat-messages') || 
							 containerRef.closest('.overflow-auto') || 
							 containerRef.closest('[data-scroll]') ||
							 containerRef.parentElement;
		
		
		// Listen to all possible scroll containers
		let currentEl = containerRef;
		while (currentEl && currentEl !== document.body) {
			const style = window.getComputedStyle(currentEl);
			if (style.overflow === 'auto' || style.overflow === 'scroll' || 
				style.overflowY === 'auto' || style.overflowY === 'scroll') {
				currentEl.addEventListener('scroll', updateButtonPositions, { passive: true });
			}
			currentEl = currentEl.parentElement;
		}
		
		// Add a test interval to see if positioning works
		const testInterval = setInterval(() => {
			updateButtonPositions();
		}, 1000);
		
		// Store interval for cleanup
		containerRef._testInterval = testInterval;
	}

	function updateButtonPositions() {
		if (!containerRef) return;
		

		const codeBlocks = containerRef.querySelectorAll('.code-block-wrapper');
		
		codeBlocks.forEach((codeBlock, index) => {
			const button = codeBlock.querySelector('.copy-button');
			if (!button) {
				return;
			}

			const blockRect = codeBlock.getBoundingClientRect();
			const containerRect = codeBlock.querySelector('.code-block-container').getBoundingClientRect();
			const viewportHeight = window.innerHeight;
			const buttonHeight = 32;
			const margin = 12;

			if (blockRect.bottom > 0 && blockRect.top < viewportHeight) {
				let targetTop = margin;

				if (containerRect.top < 0) {
					const viewportTopInContainer = Math.abs(containerRect.top);
					targetTop = viewportTopInContainer + margin;
					
					const maxTop = containerRect.height - buttonHeight - margin;
					targetTop = Math.min(targetTop, maxTop);
					
				}

				button.style.top = `${targetTop}px`;
				button.classList.add('positioned');
				
			} else {
				button.style.top = `${margin}px`;
				button.classList.remove('positioned');
			}
		});
	}

	onMount(() => {
		setupCopyButtons();
	});

	onDestroy(() => {
		copyButtons.forEach((handler, blockId) => {
			const button = containerRef?.querySelector(`[data-block-id="${blockId}"]`);
			if (button) {
				button.removeEventListener('click', handler);
			}
		});
		copyButtons.clear();

		copiedStates.forEach(timeout => clearTimeout(timeout));
		copiedStates.clear();

		if (intersectionObserver) {
			intersectionObserver.disconnect();
		}

		window.removeEventListener('scroll', updateButtonPositions);
		document.removeEventListener('scroll', updateButtonPositions);
		window.removeEventListener('resize', updateButtonPositions);
		
		// Clean up container scroll listener if it exists
		if (containerRef) {
			// Clean up test interval
			if (containerRef._testInterval) {
				clearInterval(containerRef._testInterval);
			}
			
			// Clean up scroll listeners on parent elements
			let currentEl = containerRef;
			while (currentEl && currentEl !== document.body) {
				currentEl.removeEventListener('scroll', updateButtonPositions);
				currentEl = currentEl.parentElement;
			}
		}

	});

	// Re-setup when content changes
	$: if (renderedHtml) {
		tick().then(() => {
			if (containerRef) {
				setupCopyButtons();
				// Initial position update
				setTimeout(updateButtonPositions, 100);
			}
		});
	}
</script>

<div class="markdown-content" bind:this={containerRef}>
	{@html renderedHtml}
</div>

<style>
	.markdown-content {
		color: var(--text-primary);
		line-height: var(--line-height-normal);
		white-space: normal !important;
	}

	:global(.markdown-content h1),
	:global(.markdown-content h2),
	:global(.markdown-content h3),
	:global(.markdown-content h4),
	:global(.markdown-content h5),
	:global(.markdown-content h6) {
		color: var(--text-primary);
		font-weight: 600;
		margin: 1.5em 0 0.5em 0;
		line-height: 1.2;
	}

	:global(.markdown-content h1) { font-size: 1.875rem; }
	:global(.markdown-content h2) { font-size: 1.5rem; }
	:global(.markdown-content h3) { font-size: 1.25rem; }
	:global(.markdown-content h4) { font-size: 1.125rem; }
	:global(.markdown-content h5) { font-size: 1rem; }
	:global(.markdown-content h6) { font-size: 0.875rem; }

	:global(.markdown-content p) {
		margin: 0.5em 0 !important;
		white-space: normal !important;
	}
	
	:global(.markdown-content p:first-child) {
		margin-top: 0 !important;
	}
	
	:global(.markdown-content p:last-child) {
		margin-bottom: 0 !important;
	}

	:global(.markdown-content ul),
	:global(.markdown-content ol) {
		margin: 0.5em 0 !important;
		padding-left: 1.5em !important;
	}

	:global(.markdown-content li) {
		margin: 0.125em 0 !important;
	}

	:global(.markdown-content blockquote) {
		border-left: 3px solid var(--border-primary);
		padding-left: 1rem;
		margin: 1rem 0;
		color: var(--text-secondary);
		font-style: italic;
	}

	:global(.markdown-content code) {
		background-color: var(--bg-tertiary);
		color: var(--text-primary);
		padding: 0.125rem 0.25rem;
		border-radius: var(--radius-sm);
		font-family: var(--font-family-mono);
		font-size: 0.875em;
	}

	:global(.markdown-content .code-block-wrapper) {
		position: relative;
		margin: 0.75rem 0;
		border-radius: var(--radius-lg);
		background-color: var(--bg-tertiary);
		border: 1px solid var(--border-primary);
		overflow: hidden;
	}

	:global(.markdown-content .code-block-header) {
		background-color: var(--bg-secondary);
		padding: 0.5rem 1rem;
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	:global(.markdown-content .code-block-container) {
		position: relative;
		overflow: visible;
	}

	:global(.markdown-content .code-block-wrapper pre) {
		margin: 0;
		padding: 1rem;
		overflow-x: auto;
		background-color: transparent;
		border: 0;
	}

	:global(.markdown-content .code-block-wrapper code) {
		background-color: transparent;
		padding: 0;
		border-radius: 0;
		font-size: 0.875rem;
		line-height: 1.5;
		color: var(--text-primary);
	}

	:global(.markdown-content .copy-button) {
		position: absolute;
		top: 0.75rem;
		right: 0.75rem;
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-md);
		padding: 0.5rem;
		color: var(--text-secondary);
		cursor: pointer;
		opacity: 0;
		transition: opacity 0.2s ease, transform 0.15s ease;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		z-index: 10;
		backdrop-filter: blur(4px);
		-webkit-backdrop-filter: blur(4px);
	}

	:global(.markdown-content .code-block-wrapper:hover .copy-button),
	:global(.markdown-content .copy-button.visible),
	:global(.markdown-content .copy-button.positioned) {
		opacity: 1 !important;
	}

	:global(.markdown-content .copy-button:hover) {
		background-color: var(--bg-primary);
		border-color: var(--border-secondary);
		color: var(--text-primary);
		transform: scale(1.05);
	}

	:global(.markdown-content .copy-button.copied) {
		background-color: var(--accent-primary-muted);
		border-color: var(--accent-primary);
		color: var(--accent-primary);
	}

	:global(.markdown-content .copy-icon),
	:global(.markdown-content .check-icon) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}

	:global(.markdown-content a) {
		color: var(--accent-primary);
		text-decoration: none;
	}

	:global(.markdown-content a:hover) {
		text-decoration: underline;
	}

	:global(.markdown-content table) {
		border-collapse: collapse;
		width: 100%;
		margin: 1rem 0;
	}

	:global(.markdown-content th),
	:global(.markdown-content td) {
		border: 1px solid var(--border-primary);
		padding: 0.5rem;
		text-align: left;
	}

	:global(.markdown-content th) {
		background-color: var(--bg-secondary);
		font-weight: 600;
	}

	/* Syntax highlighting styles - Theme-aware */
	:global(.hljs) {
		background: transparent !important;
	}

	/* Kawaii colors for dark mode */
	:global([data-theme="dark"] .hljs-comment),
	:global([data-theme="dark"] .hljs-quote) {
		color: #b8b8b8;
		font-style: italic;
	}

	:global([data-theme="dark"] .hljs-keyword),
	:global([data-theme="dark"] .hljs-selector-tag),
	:global([data-theme="dark"] .hljs-literal) {
		color: #f9a3ff;
		font-weight: 600;
	}

	:global([data-theme="dark"] .hljs-string) {
		color: #a3f9d3;
	}

	:global([data-theme="dark"] .hljs-number) {
		color: #a3d9ff;
	}

	:global([data-theme="dark"] .hljs-built_in),
	:global([data-theme="dark"] .hljs-class .hljs-title) {
		color: #ffa3d9;
	}

	:global([data-theme="dark"] .hljs-function .hljs-title) {
		color: #fff9a3;
		font-weight: 600;
	}

	:global([data-theme="dark"] .hljs-tag) {
		color: #f9a3ff;
	}

	:global([data-theme="dark"] .hljs-name) {
		color: #d9a3ff;
	}

	:global([data-theme="dark"] .hljs-attribute) {
		color: #a3fff9;
	}

	:global([data-theme="dark"] .hljs-variable) {
		color: #e4e4e4;
	}

	:global([data-theme="dark"] .hljs-type) {
		color: #ffa3d9;
	}

	:global([data-theme="dark"] .hljs-symbol),
	:global([data-theme="dark"] .hljs-meta) {
		color: #ffd9a3;
	}

	:global([data-theme="dark"] .hljs-operator) {
		color: #ffb3a3;
	}

	:global([data-theme="dark"] .hljs-punctuation) {
		color: #c8c8c8;
	}

	/* Kawaii colors for light mode */
	:global([data-theme="light"] .hljs-comment),
	:global([data-theme="light"] .hljs-quote) {
		color: #777777;
		font-style: italic;
	}

	:global([data-theme="light"] .hljs-keyword),
	:global([data-theme="light"] .hljs-selector-tag),
	:global([data-theme="light"] .hljs-literal) {
		color: #e066ff;
		font-weight: 600;
	}

	:global([data-theme="light"] .hljs-string) {
		color: #33cc88;
	}

	:global([data-theme="light"] .hljs-number) {
		color: #3399ff;
	}

	:global([data-theme="light"] .hljs-built_in),
	:global([data-theme="light"] .hljs-class .hljs-title) {
		color: #ff3399;
	}

	:global([data-theme="light"] .hljs-function .hljs-title) {
		color: #cc9900;
		font-weight: 600;
	}

	:global([data-theme="light"] .hljs-tag) {
		color: #e066ff;
	}

	:global([data-theme="light"] .hljs-name) {
		color: #9966ff;
	}

	:global([data-theme="light"] .hljs-attribute) {
		color: #00ccaa;
	}

	:global([data-theme="light"] .hljs-variable) {
		color: #333333;
	}

	:global([data-theme="light"] .hljs-type) {
		color: #ff3399;
	}

	:global([data-theme="light"] .hljs-symbol),
	:global([data-theme="light"] .hljs-meta) {
		color: #ff9933;
	}

	:global([data-theme="light"] .hljs-operator) {
		color: #ff6633;
	}

	:global([data-theme="light"] .hljs-punctuation) {
		color: #666666;
	}

	/* Default theme-aware colors for all other themes */
	:global(.hljs-comment),
	:global(.hljs-quote) {
		color: var(--text-muted);
		font-style: italic;
	}

	:global(.hljs-keyword),
	:global(.hljs-selector-tag),
	:global(.hljs-literal) {
		color: var(--accent-primary);
		font-weight: 600;
	}

	:global(.hljs-string) {
		color: var(--status-success);
	}

	:global(.hljs-number) {
		color: var(--status-info);
	}

	:global(.hljs-built_in),
	:global(.hljs-class .hljs-title) {
		color: var(--accent-secondary);
	}

	:global(.hljs-function .hljs-title) {
		color: var(--status-warning);
		font-weight: 600;
	}

	:global(.hljs-tag) {
		color: var(--accent-primary);
	}

	:global(.hljs-name) {
		color: var(--text-secondary);
	}

	:global(.hljs-attribute) {
		color: var(--text-tertiary);
	}

	:global(.hljs-variable) {
		color: var(--text-primary);
	}

	:global(.hljs-type) {
		color: var(--accent-secondary);
	}

	:global(.hljs-symbol),
	:global(.hljs-meta) {
		color: var(--text-tertiary);
	}

	:global(.hljs-operator) {
		color: var(--text-secondary);
	}

	:global(.hljs-punctuation) {
		color: var(--text-secondary);
	}
</style>
