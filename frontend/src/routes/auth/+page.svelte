<script>
  import ASCIIArt from '$lib/components/ASCIIArt.svelte';
  import AuthForm from '$lib/components/AuthForm.svelte';
</script>

<svelte:head>
  <title>Sign In - neko.chat</title>
  <meta name="description" content="Sign in to neko.chat - Terminal-native intelligence system" />
</svelte:head>

<div class="auth-page">
  <div class="auth-container">
    <div class="left-column">
      <ASCIIArt />
    </div>
    
    <div class="right-column">
      <AuthForm />
    </div>
  </div>
</div>

<style>
  .auth-page {
    background-color: var(--bg-primary);
    display: flex;
    height: 100vh;
    align-items: center;
    justify-content: center;
  }

  .auth-container {
    width: 100%;
    display: grid;
    height: 100vh;

    grid-template-columns: 1fr 1fr;
    background-color: var(--bg-secondary);
    overflow: hidden;
    backdrop-filter: blur(10px);
  }

  .left-column {

    position: relative;
    overflow: hidden;
  }



  .right-column {
    background-color: var(--bg-primary);
    position: relative;
  }



  /* Responsive design */
  @media (max-width: 1024px) {
    .auth-container {
      max-height: 700px;
    }
  }

  @media (max-width: 768px) {
    .auth-page {
      padding: var(--spacing-md);
    }
    
    .auth-container {
      grid-template-columns: 1fr;
      max-height: none;
      height: auto;
      min-height: 600px;
    }
    
    .left-column {
      border-right: none;
      border-bottom: 1px solid var(--border-primary);
      min-height: 200px;
    }
    
    .right-column::before {
      display: none;
    }
  }

  @media (max-width: 640px) {
    .auth-container {
      border-radius: var(--radius-lg);
      min-height: 100vh;
      height: 100vh;
    }
    
    .left-column {
      display: none;
    }
    
    .auth-container {
      grid-template-columns: 1fr;
    }
  }


  /* Subtle animation */
  .auth-container {
    animation: fade-in 0.6s ease-out;
  }

  @keyframes fade-in {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
