<script>
  import { createEventDispatcher, onMount } from "svelte";
  import { ZoomOut } from "lucide-svelte";

  const dispatch = createEventDispatcher();

  export let chatTree = {};
  export let activeChatId = null;

  let svgElement;
  let container;
  let nodes = [];
  let links = [];
  let transform = { x: 0, y: 0, scale: 1 };
  let isDragging = false;
  let dragStart = { x: 0, y: 0 };

  $: {
    updateGraph(chatTree);
  }

  function updateGraph(tree) {
    nodes = [];
    links = [];

    if (!tree || Object.keys(tree).length === 0) return;

    const processedNodes = new Set();

    function processNode(chatId, chat, depth = 0, x = 0) {
      if (processedNodes.has(chatId)) return;
      processedNodes.add(chatId);

      const node = {
        id: chatId,
        chat: chat,
        x: x,
        y: depth * 80 + 50,
        depth: depth,
        isActive: chatId === activeChatId,
        label: `${chat.model}`,
        provider: chat.provider,
      };

      nodes.push(node);

      if (chat.children && chat.children.length > 0) {
        const childSpacing = 200;
        const startX = x - ((chat.children.length - 1) * childSpacing) / 2;

        chat.children.forEach((child, index) => {
          const childX = startX + index * childSpacing;
          processNode(child.id, child, depth + 1, childX);

          links.push({
            source: chatId,
            target: child.id,
          });
        });
      }
    }

    const rootChat = Object.values(tree).find((chat) => !chat.parentChatId);
    if (rootChat) {
      processNode(rootChat.id, rootChat);
    }

    nodes = [...nodes];
    links = [...links];
  }

  function handleNodeClick(node) {
    dispatch("nodeClick", { chatId: node.id, chat: node.chat });
  }

  function getNodeAsset(node, assetType) {
    const provider = (node.provider || "").toLowerCase();
    const modelId = (node.chat.model || "").toLowerCase();

    const assets = {
      openai: { logo: "/openai.svg", color: "#10a37f" },
      anthropic: { logo: "/claude.svg", color: "#d4a574" },
      google: { logo: "/google.svg", color: "#4285F4" },
      gemini: { logo: "/google.svg", color: "#4285F4" },
      openrouter: { logo: "/openrouter.svg", color: "#3B82F6" },
      xai: { logo: null, color: "#1DA1F2" },
      default: { logo: null, color: "var(--text-secondary)" },
    };

    switch (provider) {
      case "openai":
        return assets.openai[assetType];
      case "anthropic":
        return assets.anthropic[assetType];
      case "google":
        return assets.google[assetType];
      case "gemini":
        return assets.gemini[assetType];
      case "xai":
        return assets.xai[assetType];
      case "openrouter":
        return assets.openrouter[assetType];
      default:
        return assets.default[assetType];
    }
  }

  function handleMouseDown(event) {
    if (event.button === 0) {
      isDragging = true;
      dragStart = { x: event.clientX, y: event.clientY };
      event.preventDefault();
    }
  }

  function handleMouseMove(event) {
    if (isDragging) {
      const deltaX = event.clientX - dragStart.x;
      const deltaY = event.clientY - dragStart.y;
      transform = {
        ...transform,
        x: transform.x + deltaX,
        y: transform.y + deltaY,
      };
      dragStart = { x: event.clientX, y: event.clientY };
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }

  function handleWheel(event) {
    event.preventDefault();
    const delta = event.deltaY > 0 ? 0.9 : 1.1;
    const newScale = Math.min(Math.max(transform.scale * delta, 0.1), 3);
    const rect = svgElement.getBoundingClientRect();
    const mouseX = event.clientX - rect.left;
    const mouseY = event.clientY - rect.top;
    const factor = newScale / transform.scale;
    transform = {
      x: mouseX - (mouseX - transform.x) * factor,
      y: mouseY - (mouseY - transform.y) * factor,
      scale: newScale,
    };
  }

  function resetView() {
    transform = { x: 0, y: 0, scale: 1 };
  }

  onMount(() => {
    const handleGlobalMouseMove = (event) => handleMouseMove(event);
    const handleGlobalMouseUp = () => handleMouseUp();
    document.addEventListener("mousemove", handleGlobalMouseMove);
    document.addEventListener("mouseup", handleGlobalMouseUp);
    return () => {
      document.removeEventListener("mousemove", handleGlobalMouseMove);
      document.removeEventListener("mouseup", handleGlobalMouseUp);
    };
  });
</script>

<div class="chat-graph" bind:this={container}>
  {#if nodes.length > 0}
    <div class="graph-controls">
      <button class="control-btn" on:click={resetView} title="Reset view">
        <ZoomOut size={16} />
      </button>
    </div>

    <svg
      class="graph-svg"
      viewBox="-400 -200 800 600"
      bind:this={svgElement}
      on:mousedown={handleMouseDown}
      on:wheel={handleWheel}
      style="cursor: {isDragging ? 'grabbing' : 'grab'}"
    >
      <g
        transform="translate({transform.x}, {transform.y}) scale({transform.scale})"
      >
        <!-- Links -->
        {#each links as link}
          {@const sourceNode = nodes.find((n) => n.id === link.source)}
          {@const targetNode = nodes.find((n) => n.id === link.target)}
          {#if sourceNode && targetNode}
            <line
              x1={sourceNode.x}
              y1={sourceNode.y}
              x2={targetNode.x}
              y2={targetNode.y}
              stroke="var(--border-color)"
              stroke-width="2"
            />
          {/if}
        {/each}

        <!-- Nodes -->
        {#each nodes as node}
          {@const logo = getNodeAsset(node, "logo")}
          {@const color = node.isActive
            ? "var(--accent-color)"
            : getNodeAsset(node, "color")}
          <g
            class="node-group"
            on:click={() => handleNodeClick(node)}
            role="button"
            tabindex="0"
            style="cursor: pointer"
          >
            <circle
              cx={node.x}
              cy={node.y}
              r="20"
              fill={color}
              stroke="var(--bg-primary)"
              stroke-width="2"
              class="node-circle"
              class:active={node.isActive}
            />
            {#if logo}
              <image
                href={logo}
                x={node.x - 12}
                y={node.y - 12}
                width="24"
                height="24"
                class="node-logo"
                style="pointer-events: none;"
              />
            {/if}
            <text
              x={node.x}
              y={node.y + 35}
              text-anchor="middle"
              fill="var(--text-primary)"
              font-size="12"
              class="node-label"
              style="pointer-events: none"
            >
              {node.label}
            </text>
          </g>
        {/each}
      </g>
    </svg>
  {:else}
    <div class="empty-state">
      <div class="empty-icon">💬</div>
      <p>Start chatting to see your conversation tree</p>
      <span class="empty-hint">Select multiple models to create branches</span>
    </div>
  {/if}
</div>

<style>
  .chat-graph {
    width: 100%;
    height: 100%;
    background: var(--bg-primary);
    overflow: hidden;
    position: relative;
    user-select: none;
  }

  .graph-controls {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    z-index: 10;
  }

  .control-btn {
    padding: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--bg-secondary);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .control-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .graph-svg {
    width: 100%;
    height: 100%;
    background: var(--bg-primary);
  }

  .node-circle {
    transition: all 0.2s ease;
  }

  .node-group:hover .node-circle {
    r: 25;
    stroke-width: 3;
  }

  .node-circle.active {
    stroke: var(--accent-color);
    stroke-width: 3;
  }

  .node-label {
    font-family: inherit;
    font-weight: 500;
    font-size: 11px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    text-align: center;
    padding: 2rem;
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
    opacity: 0.5;
  }

  .empty-state p {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
    color: var(--text-primary);
  }

  .empty-hint {
    font-size: 0.875rem;
    color: var(--text-secondary);
    opacity: 0.8;
  }
</style>
