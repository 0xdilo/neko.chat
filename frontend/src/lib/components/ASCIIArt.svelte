<script>
  // Sophisticated abstract ASCII art for the auth page
  const asciiArt = `

        ████                      ████        
      ██    ██                  ██    ██      
      ██    ██                  ██    ██      
    ██        ██████████████████        ██    
    ██        ▓▓▓▓  ▓▓▓▓▓▓  ▓▓▓▓        ██    
    ██        ▓▓▓▓  ▓▓▓▓▓▓  ▓▓▓▓        ██    
  ██                                      ██  
  ██  ██    ████              ████    ██  ██  
  ██    ██  ████      ██      ████  ██    ██  
██    ██            ██████            ██    ██
██                                          ██
██                                          ██
██▓▓▓▓                                  ▓▓▓▓██
██▓▓▓▓                                  ▓▓▓▓██
██                                          ██

███╗   ██╗    ███████╗    ██╗  ██╗     ██████╗ 
████╗  ██║    ██╔════╝    ██║ ██╔╝    ██╔═══██╗
██╔██╗ ██║    █████╗      █████╔╝     ██║   ██║
██║╚██╗██║    ██╔══╝      ██╔═██╗     ██║   ██║
██║ ╚████║    ███████╗    ██║  ██╗    ╚██████╔╝
╚═╝  ╚═══╝    ╚══════╝    ╚═╝  ╚═╝     ╚═════╝ .chat


  `;
</script>

<div class="ascii-container">
  <pre class="ascii-art">{asciiArt}</pre>
</div>

<style>
  .ascii-container {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }

  .ascii-art {
    line-height: 1.2;
    color: var(--text-secondary);
    background: transparent;
    border: none;
    white-space: pregg;
    margin: 0;
    position: relative;
    z-index: 2;
    filter: drop-shadow(0 0 10px rgba(135, 135, 135, 0.1));
font-family: 'monospace', 'menlo', 'consolas', monospace; /* IMPORTANT: specify monospace fonts */
    white-space: pre; /* IMPORTANT: use pre for exact whitespace preservation */
    line-height: 1; /* often 1 or closer to 1 is better for pixel art */
    letter-spacing: -0.6px; /* ensure no weird kerning */
    text-shadow: none; /* drop-shadow can blur; better to control char rendering directly */
    filter: none; /* remove filter that blurs chars */
  }

  /* Responsive adjustments */
  @media (max-width: 1024px) {
    .ascii-art {
      font-size: 0.655rem;
    }
  }

  @media (max-width: 768px) {
    .ascii-art {
      font-size: 0.55rem;
    }
  }

  @media (max-width: 640px) {
    .ascii-container {
      display: none;
    }
  }
</style>
