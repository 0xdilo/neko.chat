<script>
	import {
		Send,
		Bot,
		Copy,
		RefreshCw,
		GitFork,
		Pencil,
		Paperclip,
		Globe,
		X,
		PanelRight,
		ChevronDown,
		Trash2,
		Square,
	} from "lucide-svelte";
	import MarkdownRenderer from "./MarkdownRenderer.svelte";
	import WelcomeMessage from "./WelcomeMessage.svelte";
	import { onMount, onDestroy, tick } from "svelte";
	import { browser } from "$app/environment";
	import {
		activeChatMessages,
		activeChat,
		currentChat,
		chats,
		sendMessage,
		loadChats,
		createChat,
		updateChat,
		addMessageToActiveChat,
		updateMessageInActiveChat,
		sendParallelMessage,
		deleteMessage,
		buildChatTree,
	} from "$lib/stores/chats.js";
	import { showError, showSuccess } from "$lib/stores/app.js";
	import { api } from "$lib/api/client.js";
	import { chatAPI } from "$lib/api/chats.js";
	import { apiKeys } from "$lib/stores/settings.js";
	import {
		enabledModels,
		enabledModelsByProvider,
		getModelDisplayName as getModelName,
		getProviderDisplayName,
		loadEnabledModels,
		getFirstEnabledModel,
		anySelectedModelSupportsWebSearch,
		lastUsedModel,
	} from "$lib/stores/models.js";
	import { rightSidebarCollapsed, sidebarCollapsed } from "$lib/stores/ui.js";

	let messageInput = "";
	let messagesContainer;
	let fileInput;
	let webAccessEnabled = false;
	let showModelSelector = false;
	let modelSelectorButton;
	// let isLoading = false; // Replaced with per-chat loading state
	let loadingChats = new Set(); // Track which chats are currently loading
	let abortControllers = new Map(); // Track abort controllers for cancellation
	let selectedModelIndex = 0;
	let editingMessageId = null;
	let editingContent = "";
	let showBranchModal = false;
	let branchingMessageId = null;
	let branchingMessageIndex = -1;
	let showModelSelection = false;
	let selectedProvider = null;
	let selectedModelForBranch = null;
	let currentSelectedModels = [];
	let showTopModelSelector = false;
	let isFirstMessage = false;

	// scroll management state
	let isAtBottom = true;
	let showScrollButton = false;
	let scrollThreshold = 100;
	let shouldAutoScroll = true;

	// user message expansion state
	let expandedUserMessages = new Set();
	const TRUNCATE_LENGTH = 500;

	// Store per-chat model selections
	let chatModelMemory = {};
	let isRestoringChatModel = false;
	let lastChatId = null;
	let userHasManuallyDeselected = false;

	// Use reactive statements for dynamic models
	$: userEnabledModels = $enabledModels;
	$: modelsByProvider = $enabledModelsByProvider;
	$: webSearchSupported = anySelectedModelSupportsWebSearch(currentSelectedModels, userEnabledModels);
	
	// Reset web search when switching to models that don't support it
	$: if (!webSearchSupported && webAccessEnabled) {
		webAccessEnabled = false;
	}
	$: currentChatLoading = $activeChat && loadingChats.has($activeChat);
	$: availableProviders = Object.entries(modelsByProvider).map(
		([provider, models]) => ({
			name: getProviderDisplayName(provider),
			provider: provider,
			models: models.map((m) => ({
				id: m.model_id,
				name: getModelName(m),
				...m,
			})),
		}),
	);

	// Initialize with first enabled model if none selected (only on initial load, not when user deliberately deselects)
	let hasInitialized = false;
	$: if (userEnabledModels.length > 0 && currentSelectedModels.length === 0 && !isRestoringChatModel && !hasInitialized && !userHasManuallyDeselected) {
		const firstModel = userEnabledModels[0];
		currentSelectedModels = [firstModel.model_id];
		hasInitialized = true;
	}

	// When chat changes, restore the last used model for that chat
	$: if ($currentChat && $currentChat.id && $currentChat.id !== lastChatId) {
		const chatId = $currentChat.id;
		lastChatId = chatId;
		isRestoringChatModel = true;
		
		// Reset manual deselection flag for new chat
		userHasManuallyDeselected = false;
		
		// If we have memory for this chat, restore it
		if (chatModelMemory[chatId]) {
			currentSelectedModels = [...chatModelMemory[chatId]];
			// If this chat has no models selected, it means user deliberately deselected them
			if (chatModelMemory[chatId].length === 0) {
				userHasManuallyDeselected = true;
			}
		} else {
			// Use the chat's saved model if available
			if ($currentChat.provider && $currentChat.model) {
				const chatModel = userEnabledModels.find(
					m => m.provider === $currentChat.provider && m.model_id === $currentChat.model
				);
				if (chatModel) {
					currentSelectedModels = [chatModel.model_id];
					chatModelMemory[chatId] = [chatModel.model_id];
					
					// Update lastUsedModel with current chat's model
					lastUsedModel.set({
						provider: chatModel.provider,
						model: chatModel.model_id
					});
				}
			}
		}
		
		// Reset flag after a short delay
		setTimeout(() => {
			isRestoringChatModel = false;
		}, 100);
	}

	$: userApiKeys = $apiKeys;
	$: hasApiKeys = userApiKeys && userApiKeys.length > 0;

	$: messages = $activeChatMessages.map((msg) => ({
		id: msg.id,
		type: msg.role === "user" ? "user" : "bot",
		content: msg.content,
		timestamp: new Date(msg.createdAt || msg.created_at),
		streaming: msg.streaming || false,
		error: msg.error || false,
	}));

	// scroll to bottom when messages change (initial load)
	$: if (messages.length > 0 && messagesContainer && shouldAutoScroll) {
		tick().then(() => {
			scrollToBottom(false);
			shouldAutoScroll = false; // only auto-scroll once per chat load
		});
	}

	// auto-scroll during streaming only if user was at bottom
	$: if (messages.some((m) => m.streaming) && isAtBottom && messagesContainer) {
		tick().then(() => {
			scrollToBottom(false);
		});
	}

	// reset auto-scroll flag when chat changes
	$: if ($currentChat) {
		shouldAutoScroll = true;
	}

	function handleScroll() {
		if (!messagesContainer) return;

		const { scrollTop, scrollHeight, clientHeight } = messagesContainer;
		const distanceFromBottom = scrollHeight - scrollTop - clientHeight;

		isAtBottom = distanceFromBottom <= scrollThreshold;
		showScrollButton = !isAtBottom && messages.length > 0;
	}

	function scrollToBottom(smooth = true) {
		if (!messagesContainer) return;

		messagesContainer.scrollTo({
			top: messagesContainer.scrollHeight,
			behavior: smooth ? "smooth" : "auto",
		});

		isAtBottom = true;
		showScrollButton = false;
	}

	function getTopModelDisplayName() {
		if (currentSelectedModels.length === 0) return "No model selected";
		if (currentSelectedModels.length === 1) {
			const model = userEnabledModels.find(
				(m) => m.model_id === currentSelectedModels[0],
			);
			return model ? getModelName(model) : currentSelectedModels[0];
		}
		return `${currentSelectedModels.length} models (parallel)`;
	}

	function toggleTopModelSelector() {
		showTopModelSelector = !showTopModelSelector;
		if (showTopModelSelector) {
			selectedModelIndex = 0;
		}
	}

	function scrollToHighlightedItem() {
		if (!showTopModelSelector) return;

		setTimeout(() => {
			const highlightedItem = document.querySelector(".model-item.highlighted");

			if (highlightedItem) {
				highlightedItem.scrollIntoView({
					behavior: "smooth",
					block: "nearest",
					inline: "nearest",
				});
			}
		}, 0);
	}

	onMount(async () => {
		try {
			await loadChats();
			await loadEnabledModels();
			lastUsedModel.init();

			if (typeof window !== "undefined") {
				window.addEventListener("toggle-model-selector", handleToggleModelSelector);
				document.addEventListener("keydown", handleModelSelectorKeydown);
			}
		} catch (error) {
			console.error("Failed to initialize chat interface:", error);
		}
	});

	onDestroy(() => {
		if (typeof window !== "undefined") {
			window.removeEventListener(
				"toggle-model-selector",
				handleToggleModelSelector,
			);
			document.removeEventListener("keydown", handleModelSelectorKeydown);
		}
	});

	function handleToggleModelSelector() {
		showTopModelSelector = !showTopModelSelector;
		if (showTopModelSelector) {
			selectedModelIndex = 0;
		}
	}

	function handleModelSelectorKeydown(event) {
		if (!showModelSelector && !showTopModelSelector) return;

		let allModels = [];
		availableProviders.forEach((provider) => {
			allModels = [...allModels, ...provider.models.map((m) => m.id)];
		});

		const modelList = showTopModelSelector ? allModels : allModels;

		switch (event.key) {
			case "ArrowDown":
				event.preventDefault();
				event.stopPropagation();
				selectedModelIndex = (selectedModelIndex + 1) % modelList.length;
				scrollToHighlightedItem();
				break;
			case "ArrowUp":
				event.preventDefault();
				event.stopPropagation();
				selectedModelIndex =
					selectedModelIndex === 0
						? modelList.length - 1
						: selectedModelIndex - 1;
				scrollToHighlightedItem();
				break;
			case "Enter":
				event.preventDefault();
				event.stopPropagation();
				const model = modelList[selectedModelIndex];
				let newSelection;
				if (currentSelectedModels.includes(model)) {
					newSelection = currentSelectedModels.filter((m) => m !== model);
				} else {
					newSelection = [...currentSelectedModels, model];
				}
				handleModelSelectionChange(newSelection);
				break;
			case "Escape":
				event.preventDefault();
				event.stopPropagation();
				showModelSelector = false;
				showTopModelSelector = false;
				break;
		}
	}


function autoResize(node, _val) {
  const resize = () => {
    node.style.height = 'auto';
    node.style.height = node.scrollHeight + 'px';
  };

  node.addEventListener('input', resize);
  resize();               // initial

  return {
    update: resize,       // runs whenever `messageInput` changes
    destroy() {
      node.removeEventListener('input', resize);
    }
  };
}


	function clickOutside(node, handler) {
		const handleClick = (event) => {
			if (node && !node.contains(event.target) && !event.defaultPrevented) {
				handler();
			}
		};
		document.addEventListener("click", handleClick, true);
		return {
			destroy() {
				document.removeEventListener("click", handleClick, true);
			},
		};
	}

	async function sendUserMessageWithContent(content, clearInput = false) {
		const currentChatId = $activeChat;
		if (!content.trim() || loadingChats.has(currentChatId) || !hasApiKeys) return;

		if (clearInput) {
			messageInput = "";
		}
		
		// Mark this specific chat as loading
		loadingChats.add(currentChatId);
		loadingChats = loadingChats; // Trigger reactivity

		// remember user scroll position before sending
		const wasAtBottom = isAtBottom;

		if (messages.length === 0) {
			isFirstMessage = true;
		}

		try {
			// Store intended models before any chat operations that might reset currentSelectedModels
			const intendedModels = [...currentSelectedModels];
			let result;
			
			if (intendedModels.length > 1) {
				rightSidebarCollapsed.set(false);
				sidebarCollapsed.set(true);

				const modelConfigs = intendedModels.map((modelId) => {
					const model = userEnabledModels.find((m) => m.model_id === modelId);
					if (model) {
						return {
							provider: model.provider,
							model: model.model_id,
						};
					}
					for (const provider of availableProviders) {
						const foundModel = provider.models.find((m) => m.id === modelId);
						if (foundModel) {
							return {
								provider: provider.provider,
								model: modelId,
							};
						}
					}
					const defaultModel = getFirstEnabledModel();
					return { provider: defaultModel.provider, model: modelId };
				});

				result = await sendParallelMessage(content, modelConfigs, {
					onStart: (controller) => {
						abortControllers.set(currentChatId, controller);
					}
				});
			} else {
				// Pass the selected model to sendMessage for single model usage
				if (intendedModels.length === 0) {
					// No model selected, use default
					result = await sendMessage(content, { 
						webSearch: webAccessEnabled,
						onStart: (controller) => {
							abortControllers.set(currentChatId, controller);
						}
					});
				} else {
					const selectedModelId = intendedModels[0];
					const selectedModel = userEnabledModels.find((m) => m.model_id === selectedModelId);
					
					const modelOptions = selectedModel ? {
						provider: selectedModel.provider,
						model: selectedModel.model_id,
						webSearch: webAccessEnabled,
						onStart: (controller) => {
							abortControllers.set(currentChatId, controller);
						}
					} : {
						webSearch: webAccessEnabled,
						onStart: (controller) => {
							abortControllers.set(currentChatId, controller);
						}
					};
					
					result = await sendMessage(content, modelOptions);
				}
			}

			// only auto-scroll if user was at bottom when they sent message (unless aborted)
			if (wasAtBottom && !result?.aborted) {
				await tick();
				scrollToBottom();
			}
		} catch (error) {
			console.error("Failed to send message:", error);
			// Don't show error if it was just an aborted stream
			if (error.name !== 'AbortError') {
				showError("Failed to send message");
			}
		} finally {
			// Remove this chat from loading state
			loadingChats.delete(currentChatId);
			abortControllers.delete(currentChatId);
			loadingChats = loadingChats; // Trigger reactivity
			isFirstMessage = false;
		}
	}

	async function sendUserMessage() {
		const content = messageInput.trim();
		if (!content) return;
		await sendUserMessageWithContent(content, true);
	}

	// Handler for welcome message suggestions
	async function handleWelcomeMessage(content) {
		await sendUserMessageWithContent(content, false);
	}


async function handleKeydown(e) {
    // manual newline -------------------------------------------------
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();               // stop parent handlers too
      const el = e.target;
      const { selectionStart: s, selectionEnd: ePos } = el;
      messageInput =
        messageInput.slice(0, s) + '\n' + messageInput.slice(ePos);
      await tick();                     // wait for bind:value to update
      el.selectionStart = el.selectionEnd = s + 1; // caret after \n
      return;
    }

    // normal send ----------------------------------------------------
    if (
      e.key === 'Enter' &&
      !e.shiftKey &&
      !e.ctrlKey &&
      !e.metaKey &&
      !showModelSelector &&
      !showTopModelSelector &&
      document.activeElement === e.target
    ) {
      e.preventDefault();
      sendUserMessage();
    }
  }



	const formatTime = (date) =>
		date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
	const copyContent = (content) => navigator.clipboard.writeText(content);
	const editMessage = (messageId, content) => {
		editingMessageId = messageId;
		editingContent = content;
	};

	const cancelEdit = () => {
		editingMessageId = null;
		editingContent = "";
	};

	const saveEditedMessage = async () => {
		if (!editingMessageId || !editingContent.trim() || !$currentChat) return;

		const wasAtBottom = isAtBottom;

		try {
			const messageIndex = $activeChatMessages.findIndex(
				(msg) => msg.id === editingMessageId,
			);
			if (messageIndex === -1) return;

			await chatAPI.updateMessage($currentChat.id, editingMessageId, {
				content: editingContent.trim(),
			});

			await chatAPI.deleteSubsequentMessages($currentChat.id, editingMessageId);

			const updatedMessages = await chatAPI.getMessages($currentChat.id);
			activeChatMessages.set(updatedMessages);

			const originalMessage = $activeChatMessages.find(
				(msg) => msg.id === editingMessageId,
			);
			const wasUserMessage = originalMessage && originalMessage.role === "user";

			cancelEdit();

			if (wasUserMessage) {
				const assistantMessageId = `assistant-${Date.now()}`;
				const assistantMessage = {
					id: assistantMessageId,
					role: "assistant",
					content: "",
					created_at: new Date().toISOString(),
					streaming: true,
				};

				addMessageToActiveChat(assistantMessage);

				try {
					await chatAPI.regenerateResponse($currentChat.id, {
						onStart: (controller) => {
							abortControllers.set($currentChat.id, controller);
						},
						onChunk: (chunk, accumulatedContent) => {
							updateMessageInActiveChat(assistantMessageId, {
								content: accumulatedContent,
								streaming: true,
							});
						},
						onComplete: async () => {
							updateMessageInActiveChat(assistantMessageId, {
								streaming: false,
							});
							abortControllers.delete($currentChat.id);

							try {
								const messages = await chatAPI.getMessages($currentChat.id);
								activeChatMessages.set(messages);
							} catch (err) {
								console.warn("Failed to reload messages after editing:", err);
							}
						},
						onError: (error) => {
							console.error("Streaming error:", error);
							abortControllers.delete($currentChat.id);
							updateMessageInActiveChat(assistantMessageId, {
								content: "Error: Failed to get response",
								streaming: false,
								error: true,
							});
							showError("Failed to get response");
						},
					});
				} catch (error) {
					console.error("Failed to regenerate response after editing:", error);
					updateMessageInActiveChat(assistantMessageId, {
						content: "Error: Failed to send message",
						streaming: false,
						error: true,
					});
					showError("Failed to send message");
				}
			}

			if (wasAtBottom) {
				await tick();
				scrollToBottom();
			}
		} catch (error) {
			console.error("Failed to save edited message:", error);
			showError("Failed to save edited message");
		}
	};

	const handleEditKeydown = (event) => {
		if (event.key === "Enter" && !event.shiftKey) {
			event.preventDefault();
			saveEditedMessage();
		} else if (event.key === "Escape") {
			event.preventDefault();
			cancelEdit();
		}
	};

	const retryGeneration = async (messageId) => {
		if (!$currentChat) return;

		const wasAtBottom = isAtBottom;

		try {
			const messageIndex = $activeChatMessages.findIndex(
				(msg) => msg.id === messageId,
			);
			if (messageIndex === -1) return;

			const message = $activeChatMessages[messageIndex];

			if (message.role !== "user") return;

			await chatAPI.deleteSubsequentMessages($currentChat.id, messageId);

			const updatedMessages = await chatAPI.getMessages($currentChat.id);
			activeChatMessages.set(updatedMessages);

			const assistantMessageId = `assistant-${Date.now()}`;
			const assistantMessage = {
				id: assistantMessageId,
				role: "assistant",
				content: "",
				created_at: new Date().toISOString(),
				streaming: true,
			};

			addMessageToActiveChat(assistantMessage);

			await chatAPI.regenerateResponse($currentChat.id, {
				onStart: (controller) => {
					abortControllers.set($currentChat.id, controller);
				},
				onChunk: (chunk, accumulatedContent) => {
					updateMessageInActiveChat(assistantMessageId, {
						content: accumulatedContent,
						streaming: true,
					});
				},
				onComplete: async () => {
					updateMessageInActiveChat(assistantMessageId, {
						streaming: false,
					});
					abortControllers.delete($currentChat.id);

					try {
						const messages = await chatAPI.getMessages($currentChat.id);
						activeChatMessages.set(messages);
					} catch (err) {
						console.warn("Failed to reload messages after retry:", err);
					}
				},
				onError: (error) => {
					console.error("Streaming error:", error);
					abortControllers.delete($currentChat.id);
					updateMessageInActiveChat(assistantMessageId, {
						content: "Error: Failed to get response",
						streaming: false,
						error: true,
					});
					showError("Failed to get response");
				},
			});

			if (wasAtBottom) {
				await tick();
				scrollToBottom();
			}
		} catch (error) {
			console.error("Failed to retry message:", error);
			showError("Failed to retry message");
		}
	};

	const forkConversation = (messageId) => {
		const messageIndex = $activeChatMessages.findIndex(
			(msg) => msg.id === messageId,
		);
		if (messageIndex === -1) return;

		branchingMessageId = messageId;
		branchingMessageIndex = messageIndex;
		showBranchModal = true;
	};

	const closeBranchModal = () => {
		showBranchModal = false;
		branchingMessageId = null;
		branchingMessageIndex = -1;
	};

	const showModelSelectionModal = () => {
		showModelSelection = true;
	};

	const selectModel = (provider, model) => {
		selectedProvider = provider;
		selectedModelForBranch = model;
	};

	const confirmModelSelection = () => {
		const selectedModel = {
			provider: selectedProvider.toLowerCase(),
			model: selectedModelForBranch,
		};
		createBranch("model", selectedModel);
		showModelSelection = false;
	};

	const cancelModelSelection = () => {
		showModelSelection = false;
		selectedProvider = null;
		selectedModelForBranch = null;
	};

	const createBranch = async (branchType, selectedModel = null) => {
		if (branchingMessageIndex === -1) return;

		try {
			const messagesToInclude = $activeChatMessages.slice(
				0,
				branchingMessageIndex + 1,
			);
			const lastMessage = messagesToInclude[messagesToInclude.length - 1];

			const defaultModel = getFirstEnabledModel();
			const branchPointMessage = $activeChatMessages[branchingMessageIndex];
			const chatData = {
				title: $currentChat?.title || "Chat",
				system_prompt:
					$currentChat?.system_prompt || "You are a helpful AI assistant.",
				provider: selectedModel?.provider || $currentChat?.provider || defaultModel.provider,
				model: selectedModel?.model || $currentChat?.model || defaultModel.model,
				is_branch: true,
				parent_chat_id: $currentChat?.id,
				branch_point_message_id: branchPointMessage?.id,
			};

			closeBranchModal();

			const newChat = await createChat(chatData);

			if (messagesToInclude.length > 0) {
				const messagesToPersist =
					lastMessage.role === "user"
						? messagesToInclude.slice(0, -1)
						: messagesToInclude;

				if (messagesToPersist.length > 0) {
					await chatAPI.bulkInsertMessages(newChat.id, messagesToPersist);
				}

				const persistedMessages = await chatAPI.getMessages(newChat.id);
				activeChatMessages.set(persistedMessages);

				if (lastMessage.role === "user") {
					try {
						await sendMessage(lastMessage.content);
					} catch (sendError) {
						console.error("Failed to send user message:", sendError);
						showError("Failed to generate response for branched message");
					}
				}
			}

			showSuccess("Branch created successfully");
			
			// Refresh chat list and rebuild tree to show the new branch relationship
			const chatList = await chatAPI.getChats();
			chats.set(chatList);
			buildChatTree(chatList);
			
			// Open right sidebar to show the graph like parallel chat
			rightSidebarCollapsed.set(false);
		} catch (error) {
			console.error("Failed to create branch:", error);
			showError("Failed to create branch");
		}
	};

	const handleFileSelect = () => fileInput.click();
	const toggleModelSelector = () => (showModelSelector = !showModelSelector);

	const handleDeleteMessage = async (messageId) => {
		try {
			await deleteMessage(messageId);
		} catch (error) {
			console.error("Failed to delete message:", error);
		}
	};

	const stopGeneration = () => {
		const currentChatId = $activeChat;
		const controller = abortControllers.get(currentChatId);
		if (controller) {
			controller.abort();
			abortControllers.delete(currentChatId);
			loadingChats.delete(currentChatId);
			loadingChats = loadingChats; // Trigger reactivity
		}
	};

	// Update chat model in database
	const updateChatModel = async (chatId, provider, model) => {
		try {
			await updateChat(chatId, {
				provider,
				model
			});
		} catch (error) {
			console.error("Failed to update chat model:", error);
		}
	};

	// Handle manual model selection changes
	const handleModelSelectionChange = (newSelectedModels) => {
		currentSelectedModels = newSelectedModels;
		
		// Track if user manually deselected all models
		if (newSelectedModels.length === 0) {
			userHasManuallyDeselected = true;
		}
		
		// Save to memory for current chat
		if ($currentChat && $currentChat.id) {
			const chatId = $currentChat.id;
			chatModelMemory[chatId] = [...newSelectedModels];
			
			// Update chat model if single model selected
			if (newSelectedModels.length === 1) {
				const selectedModel = userEnabledModels.find(m => m.model_id === newSelectedModels[0]);
				if (selectedModel && 
					(selectedModel.provider !== $currentChat.provider || 
					 selectedModel.model_id !== $currentChat.model)) {
					updateChatModel(chatId, selectedModel.provider, selectedModel.model_id);
				}
			}
		}
		
		// Update last used model for new chat creation (only for single model selection)
		if (newSelectedModels.length === 1) {
			const selectedModel = userEnabledModels.find(m => m.model_id === newSelectedModels[0]);
			if (selectedModel) {
				lastUsedModel.set({
					provider: selectedModel.provider,
					model: selectedModel.model_id
				});
			}
		}
	};
</script>

<div class="chat-interface">
	<div class="chat-header">
		<div class="header-left">
			<div class="model-selector-top">
				<button
					class="model-selector-button"
					on:click={toggleTopModelSelector}
					class:active={showTopModelSelector}
				>
					<span class="model-name">{getTopModelDisplayName()}</span>
					<svg
						class="chevron"
						class:rotated={showTopModelSelector}
						width="16"
						height="16"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<polyline points="6,9 12,15 18,9"></polyline>
					</svg>
				</button>

				{#if showTopModelSelector}
					<div
						class="model-selector-dropdown"
						use:clickOutside={() => (showTopModelSelector = false)}
						on:click|stopPropagation
						role="menu"
						tabindex="-1"
					>
						<div class="dropdown-header">
							Select Models ({currentSelectedModels.length} selected)
						</div>

						<div class="models-list">
							{#each availableProviders as provider}
								{#if provider.models.length > 0}
									<div class="provider-group">
										<div class="provider-label">{provider.name}</div>
										{#each provider.models as model, index}
											{@const globalIndex =
												availableProviders
													.slice(0, availableProviders.indexOf(provider))
													.reduce((acc, p) => acc + p.models.length, 0) + index}
											<label
												class="model-item"
												class:selected={currentSelectedModels.includes(model.id)}
												class:highlighted={globalIndex === selectedModelIndex}
												on:click={(e) => {
													e.preventDefault();
													const modelId = model.id;
													let newSelection;
													if (currentSelectedModels.includes(modelId)) {
														// Model is currently selected, remove it
														newSelection = currentSelectedModels.filter(m => m !== modelId);
													} else {
														// Model is not selected, add it
														newSelection = [...currentSelectedModels, modelId];
													}
													handleModelSelectionChange(newSelection);
												}}
											>
												<input
													type="checkbox"
													checked={currentSelectedModels.includes(model.id)}
													value={model.id}
													class="sr-only"
													tabindex="-1"
												/>
												<span class="model-name">{model.name}</span>
												<div
													class="check-mark"
													class:visible={currentSelectedModels.includes(model.id)}
												>
													<svg
														width="16"
														height="16"
														viewBox="0 0 20 20"
														fill="currentColor"
													>
														<path
															fill-rule="evenodd"
															d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
															clip-rule="evenodd"
														/>
													</svg>
												</div>
											</label>
										{/each}
									</div>
								{/if}
							{/each}
						</div>
					</div>
				{/if}
			</div>
		</div>
	</div>

	<div
		class="messages-container"
		bind:this={messagesContainer}
		on:scroll={handleScroll}
	>
		{#if messages.length === 0 && !isFirstMessage}
			<WelcomeMessage onSendMessage={handleWelcomeMessage} />
		{:else}
			<div class="messages">
				{#each messages as message (message.id)}
					<div
						class="message"
						class:user={message.type === "user"}
						class:bot={message.type === "bot"}
					>
						<div class="message-content">
							{#if editingMessageId === message.id}
								<div class="message-edit-container">
									<textarea
										bind:value={editingContent}
										on:keydown={handleEditKeydown}
										use:autoResize
										class="message-edit-input"
										placeholder="Edit your message..."
										autofocus
									></textarea>
									<div class="edit-actions">
										<button
											class="edit-action-button save"
											on:click={saveEditedMessage}>Save</button
										>
										<button
											class="edit-action-button cancel"
											on:click={cancelEdit}>Cancel</button
										>
									</div>
								</div>
							{:else}
								{#if message.type === "bot"}
									<div
										class="message-text"
										class:streaming={message.streaming}
										class:error={message.error}
									>
										<MarkdownRenderer content={message.content} />
										{#if message.streaming && !message.content}
											<span class="cursor-blink">▋</span>
										{/if}
									</div>
								{:else}
									{@const isLong = message.content.length > TRUNCATE_LENGTH}
									{@const isExpanded = expandedUserMessages.has(message.id)}
									{#if !isLong}
										<div class="message-text">
											{message.content}
										</div>
									{:else}
										<div
											class="message-text expandable"
											on:click={() => {
												isExpanded
													? expandedUserMessages.delete(message.id)
													: expandedUserMessages.add(message.id);
												expandedUserMessages = expandedUserMessages;
											}}
										>
											<div class="user-content-wrapper" class:truncated={!isExpanded}>
												{#if !isExpanded}
													{message.content.substring(0, TRUNCATE_LENGTH)}…
												{:else}
													{message.content}
												{/if}
											</div>
											<div class="expand-button" class:expanded={isExpanded}>
												<ChevronDown size={16} />
											</div>
										</div>
									{/if}
								{/if}
							{/if}
							<div class="message-footer">
								<div class="message-actions">
									{#if message.type === "bot"}
										<button
											class="action-button"
											on:click={() => copyContent(message.content)}
											title="Copy message"
											><Copy size={16} /></button
										>
										<button
											class="action-button"
											on:click={() => forkConversation(message.id)}
											title="Fork conversation"
											><GitFork size={16} /></button
										>
										<button
											class="action-button delete-button"
											on:click={() => handleDeleteMessage(message.id)}
											title="Delete message"
											><Trash2 size={16} /></button
										>
									{:else}
										<button
											class="action-button"
											on:click={() => copyContent(message.content)}
											title="Copy message"
											><Copy size={16} /></button
										>
										<button
											class="action-button"
											on:click={() => editMessage(message.id, message.content)}
											title="Edit message"
											><Pencil size={16} /></button
										>
										<button
											class="action-button"
											on:click={() => retryGeneration(message.id)}
											title="Retry generation"
											><RefreshCw size={16} /></button
										>
										<button
											class="action-button"
											on:click={() => forkConversation(message.id)}
											title="Fork conversation"
											><GitFork size={16} /></button
										>
										<button
											class="action-button delete-button"
											on:click={() => handleDeleteMessage(message.id)}
											title="Delete message"
											><Trash2 size={16} /></button
										>
									{/if}
								</div>
								<span class="message-time">{formatTime(message.timestamp)}</span>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<div class="chat-input-area">
		{#if showScrollButton}
			<button
				class="scroll-to-bottom-btn"
				on:click={() => scrollToBottom()}
				title="Scroll to bottom"
			>
				<svg
					width="20"
					height="20"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path d="m7 13 5 5 5-5" />
					<path d="m7 6 5 5 5-5" />
				</svg>
			</button>
		{/if}
		<div class="input-wrapper">
			<button
				class="input-action-button"
				class:active={!$rightSidebarCollapsed}
				on:click={() => {
					const newValue = !$rightSidebarCollapsed;
					rightSidebarCollapsed.set(newValue);
					if (!newValue) {
						sidebarCollapsed.set(true);
					}
				}}
			>
				<GitFork size={18} />
			</button>

			{#if webSearchSupported}
				<button
					class="input-action-button"
					class:active={webAccessEnabled}
					on:click={() => (webAccessEnabled = !webAccessEnabled)}
					title="Enable web search"
				>
					<Globe size={18} />
				</button>
			{/if}

			<textarea
				bind:value={messageInput}
use:autoResize={messageInput} 
on:keydown={handleKeydown}
				placeholder={hasApiKeys
					? "Type your message... (Ctrl+I to focus)"
					: "Add an API key in Settings to start chatting"}
				class="message-input"
				id="chat-input"
				rows="1"
				disabled={!hasApiKeys}
			></textarea>

			{#if currentChatLoading}
				<button
					on:click={stopGeneration}
					class="input-action-button stop-button"
					title="Stop generation"
				>
					<Square size={20} />
				</button>
			{:else}
				<button
					on:click={sendUserMessage}
					disabled={!messageInput.trim() || !hasApiKeys}
					class="input-action-button"
				>
					<Send size={20} />
				</button>
			{/if}
		</div>
	</div>

	{#if showBranchModal}
		<div class="modal-overlay" on:click={closeBranchModal}>
			<div class="branch-modal" on:click|stopPropagation>
				<div class="modal-header">
					<h3>Branch Conversation</h3>
					<button class="modal-close" on:click={closeBranchModal}>
						<X size={20} />
					</button>
				</div>
				<div class="modal-content">
					<p>Choose how to branch this conversation:</p>
					<div class="branch-options">
						<button
							class="branch-option-button"
							on:click={() => {
								createBranch("normal");
							}}
						>
							<div class="option-content">
								<strong>Normal Branch</strong>
								<span>Duplicate conversation up to this point</span>
							</div>
						</button>
						<button
							class="branch-option-button"
							on:click={showModelSelectionModal}
						>
							<div class="option-content">
								<strong>Fork with New Model</strong>
								<span>Choose a different model for the branch</span>
							</div>
						</button>
					</div>
				</div>
			</div>
		</div>
	{/if}

	{#if showModelSelection}
		<div class="modal-overlay" on:click={cancelModelSelection}>
			<div class="branch-modal" on:click|stopPropagation>
				<div class="modal-header">
					<h3>Select Model for Branch</h3>
					<button class="modal-close" on:click={cancelModelSelection}>
						<X size={20} />
					</button>
				</div>
				<div class="modal-content">
					<p>Choose a model for the new branch:</p>
					<div class="provider-sections">
						{#each availableProviders as provider}
							<div class="provider-section">
								<h4>{provider.name}</h4>
								<div class="model-grid">
									{#each provider.models as model}
										<button
											class="model-button"
											class:selected={selectedProvider === provider.provider &&
												selectedModelForBranch === model.id}
											on:click={() => selectModel(provider.provider, model.id)}
										>
											{model.name}
										</button>
									{/each}
								</div>
							</div>
						{/each}
					</div>
					<div class="modal-actions">
						<button
							class="modal-action-button cancel"
							on:click={cancelModelSelection}>Cancel</button
						>
						<button
							class="modal-action-button confirm"
							disabled={!selectedModelForBranch}
							on:click={confirmModelSelection}
						>
							Create Branch
						</button>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.chat-interface {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background-color: var(--bg-primary);
		overflow: hidden;
		position: relative;
	}

	.messages-container {
		flex: 1;
		overflow-y: auto;
		padding: 0 var(--spacing-3xl);
		padding-top: 0;
		padding-bottom: 8rem;
	}

	.messages {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
		max-width: var(--chat-max-width, 800px);
		margin: 0 auto;
		width: 100%;
		padding-top: 5rem;
	}

	.chat-input-area {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		width: 100%;
		max-width: var(--chat-max-width, 800px);
		margin: 0 auto;
		padding: var(--spacing-md) var(--spacing-3xl);
		flex-shrink: 0;
		background: none;
		pointer-events: none;
		z-index: 10;
	}

	.message-input {
		flex: 1;
		resize: none;
		max-height: 20vh;
		background-color: transparent;
		border: none;
		padding: var(--spacing-sm) var(--spacing-xs);
		color: var(--text-primary);
		font-family: var(--font-family-sans);
		font-size: var(--font-size-base);
		line-height: 1.5;
		overflow-y: auto;
		margin-left: 10px;
	}

	.message {
		display: flex;
		gap: var(--spacing-md);
		align-items: flex-start;
	}
	.message.user {
		flex-direction: row-reverse;
	}
	.message-content {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.message.user .message-content {
		align-items: flex-end;
	}
	.message-header {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		margin-bottom: var(--spacing-xs);
	}
	.message-text {
		font-size: var(--font-size-sm);
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-xl);
		padding: var(--spacing-md);
		color: var(--text-primary);
		line-height: var(--line-height-normal);
		white-space: pre-wrap;
		word-wrap: break-word;
		word-break: break-word; /* fix for long unbroken strings */
		overflow-wrap: break-word;
		text-align: left;
		position: relative;
	}
	.message.user .message-text {
		background-color: var(--bg-tertiary);
		border-color: var(--border-secondary);
	}
	.message.bot .message-text {
		background-color: transparent;
		border-color: transparent;
		padding-left: 0;
		padding-right: 0;
	}

	.message-text.expandable {
		cursor: pointer;
	}

	.message-text.expandable .user-content-wrapper {
		padding-bottom: 2rem;
	}

	.expand-button {
		position: absolute;
		bottom: var(--spacing-sm);
		left: 50%;
		transform: translateX(-50%);
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: 50%;
		color: var(--text-secondary);
		transition: all 150ms ease;
	}

	.message-text.expandable:hover .expand-button {
		background-color: var(--bg-tertiary);
		color: var(--text-primary);
		transform: translateX(-50%) scale(1.1);
	}

	.expand-button > :global(svg) {
		transition: transform 150ms ease-out;
	}

	.expand-button.expanded > :global(svg) {
		transform: rotate(180deg);
	}

	.message-footer {
		display: flex;
		align-items: center;
		width: 100%;
		margin-top: var(--spacing-sm);
		padding: 0 var(--spacing-xs);
		opacity: 0;
		transition: opacity 150ms ease-in-out;
		min-height: 24px;
	}
	.message.bot .message-footer {
		justify-content: space-between;
	}
	.message.user .message-footer {
		justify-content: flex-end;
		gap: var(--spacing-md);
	}
	.message:hover .message-footer {
		opacity: 1;
	}
	.message-actions {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.message.user .message-actions {
		order: 2;
	}
	.message.user .message-time {
		order: 1;
	}
	.action-button {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 4px;
		border-radius: var(--radius-xl);
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background-color 150ms ease-in-out,
			color 150ms ease-in-out;
	}
	.action-button:hover {
		background-color: var(--bg-tertiary);
		color: var(--text-primary);
	}
	
	.action-button.delete-button:hover {
		background-color: var(--status-error-muted, rgba(239, 68, 68, 0.1));
		color: var(--status-error, #ef4444);
	}
	.message-time {
		font-size: var(--font-size-xs);
		color: var(--text-muted);
		font-family: var(--font-family-mono);
	}
	.input-wrapper {
		position: relative;
		display: flex;
		align-items: flex-end;
		background-color: var(--bg-secondary);
		backdrop-filter: blur(16px);
		-webkit-backdrop-filter: blur(16px);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-xl);
		padding: var(--spacing-sm);
		transition: all var(--transition-fast);
		max-width: var(--chat-max-width, 800px);
		margin: 0 auto;
		pointer-events: auto;
		box-shadow: var(--shadow-lg);
		opacity: 0.95;
	}
	.input-wrapper:focus-within {
		border-color: var(--border-focus);
		box-shadow: 0 0 0 3px rgba(135, 135, 135, 0.15);
	}
	.message-input:focus {
		outline: none;
		box-shadow: none;
	}
	.message-input::placeholder {
		color: var(--text-muted);
	}

	.message-input:disabled {
		color: var(--text-muted);
		cursor: not-allowed;
		opacity: 0.7;
	}

	.message-input:disabled::placeholder {
		color: var(--text-muted);
		opacity: 0.8;
	}
	.input-action-button {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 0;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--radius-xl);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		transition: background-color 150ms ease-in-out,
			color 150ms ease-in-out;
	}
	.input-action-button:hover {
		background-color: var(--bg-tertiary);
		color: var(--text-primary);
	}
	.input-action-button.active {
		background-color: var(--accent-primary-muted);
		color: var(--accent-primary);
	}

	.chat-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--spacing-md) var(--spacing-3xl);
		background: none;
		border: none;
		flex-shrink: 0;
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		z-index: 10;
		pointer-events: none;
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: var(--spacing-md);
		pointer-events: auto;
	}

	.model-selector-top {
		position: relative;
		pointer-events: auto;
	}

	.model-selector-button {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-md);
		background-color: var(--bg-secondary);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-xl);
		color: var(--text-primary);
		font-size: var(--font-size-sm);
		height: 2.5rem;
		font-weight: 500;
		cursor: pointer;
		transition: all var(--transition-fast);
		min-width: 120px;
		box-shadow: var(--shadow-md);
		opacity: 0.95;
	}

	.model-selector-button:hover {
		background-color: var(--interactive-hover);
		border-color: var(--border-secondary);
		box-shadow: var(--shadow-lg);
		opacity: 1;
	}

	.model-selector-button.active {
		background-color: var(--accent-primary-muted);
		border-color: var(--accent-primary);
		color: var(--accent-primary);
	}

	.model-name {
		flex: 1;
		text-align: left;
	}

	.chevron {
		transition: transform var(--transition-fast);
		flex-shrink: 0;
	}

	.chevron.rotated {
		transform: rotate(180deg);
	}

	.model-selector-dropdown {
		position: absolute;
		top: calc(100% + 8px);
		left: 0;
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-lg);
		min-width: 280px;
		max-height: 360px;
		z-index: 20;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
		overflow: hidden;
		animation: dropdown-appear 0.15s ease-out;
	}

	.dropdown-header {
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--border-primary);
		font-size: var(--font-size-sm);
		font-weight: 600;
		color: var(--text-primary);
		background-color: var(--bg-tertiary);
	}

	.models-list {
		max-height: 300px;
		overflow-y: auto;
		padding: var(--spacing-sm);
		scroll-behavior: smooth;
	}

	.provider-group {
		margin-bottom: var(--spacing-md);
	}

	.provider-group:last-child {
		margin-bottom: 0;
	}

	.provider-label {
		font-size: var(--font-size-xs);
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: var(--spacing-sm);
		padding: 0 var(--spacing-sm);
	}

	.model-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--spacing-sm) var(--spacing-md);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition: all var(--transition-fast);
		margin-bottom: var(--spacing-xs);
	}

	.model-item:hover,
	.model-item.highlighted {
		background-color: var(--bg-tertiary);
	}

	.model-item.selected {
		background-color: var(--accent-primary-muted);
		color: var(--accent-primary);
	}

	.model-item.selected .model-name {
		color: var(--accent-primary);
		font-weight: 500;
	}

	.check-mark {
		display: flex;
		align-items: center;
		opacity: 0;
		transition: opacity var(--transition-fast);
		color: var(--accent-primary);
	}

	.check-mark.visible {
		opacity: 1;
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		border: 0;
	}

	.scroll-to-bottom-btn {
		position: absolute;
		bottom: calc(95%);
		right: var(--spacing-3xl);
		background: var(--bg-secondary);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-xl);
		color: var(--text-secondary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
		box-shadow: var(--shadow-md);
		opacity: 0.95;
		transform: translateY(8px);
		animation: fade-in 0.2s ease-out forwards;
		pointer-events: auto;
	}

	.scroll-to-bottom-btn:hover {
		background: var(--interactive-hover);
		color: var(--text-primary);
		transform: translateY(0) scale(1.05);
		box-shadow: var(--shadow-lg);
	}

	.scroll-to-bottom-btn:active {
		transform: translateY(0) scale(0.95);
	}

	@keyframes fade-in {
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@keyframes dropdown-appear {
		from {
			opacity: 0;
			transform: translateY(-4px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.loading-spinner-small {
		width: 16px;
		height: 16px;
		border: 2px solid var(--border-primary);
		border-top: 2px solid var(--accent-primary);
		border-radius: 50%;
		animation: spin 1s linear infinite;
	}

	.input-action-button.loading {
		opacity: 0.7;
		cursor: not-allowed;
	}


	.cursor-blink {
		animation: blink 1s infinite;
		color: var(--accent-primary);
	}

	@keyframes blink {
		0%,
		50% {
			opacity: 1;
		}
		51%,
		100% {
			opacity: 0;
		}
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.message-text.streaming {
		border-left: 2px solid var(--accent-primary);
		padding-left: var(--spacing-md);
	}

	.message-text.error {
		border-left: 2px solid var(--status-error);
		color: var(--status-error);
		opacity: 0.8;
	}

	.message-edit-container {
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}

	.message-edit-input {
		width: 100%;
		resize: none;
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-xl);
		padding: var(--spacing-md);
		color: var(--text-primary);
		font-family: var(--font-family-sans);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-normal);
		min-height: 60px;
		overflow-y: hidden;
	}

	.message-edit-input:focus {
		outline: none;
		border-color: var(--border-focus);
		box-shadow: 0 0 0 3px rgba(135, 135, 135, 0.15);
	}

	.edit-actions {
		display: flex;
		gap: var(--spacing-sm);
		justify-content: flex-end;
	}

	.edit-action-button {
		background: transparent;
		border: 1px solid var(--border-primary);
		color: var(--text-primary);
		cursor: pointer;
		padding: var(--spacing-xs) var(--spacing-md);
		border-radius: var(--radius-lg);
		font-size: var(--font-size-sm);
		font-weight: 500;
		transition: all 150ms ease-in-out;
	}

	.edit-action-button:hover {
		background-color: var(--bg-tertiary);
	}

	.edit-action-button.save {
		background-color: var(--accent-primary);
		color: white;
		border-color: var(--accent-primary);
	}

	.edit-action-button.save:hover {
		background-color: var(--accent-primary-dark, var(--accent-primary));
		opacity: 0.9;
	}

	.edit-action-button.cancel:hover {
		background-color: var(--bg-tertiary);
		border-color: var(--border-secondary);
	}

	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background-color: rgba(0, 0, 0, 0.5);
		backdrop-filter: blur(4px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.branch-modal {
		background-color: var(--bg-secondary);
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-xl);
		width: 90%;
		max-width: 480px;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--spacing-lg);
		border-bottom: 1px solid var(--border-primary);
	}

	.modal-header h3 {
		margin: 0;
		font-size: var(--font-size-lg);
		font-weight: 600;
		color: var(--text-primary);
	}

	.modal-close {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		padding: var(--spacing-xs);
		border-radius: var(--radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 150ms ease-in-out;
	}

	.modal-close:hover {
		background-color: var(--bg-tertiary);
		color: var(--text-primary);
	}

	.modal-content {
		padding: var(--spacing-lg);
	}

	.modal-content p {
		margin: 0 0 var(--spacing-lg) 0;
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
	}

	.branch-options {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
	}

	.branch-option-button {
		background: transparent;
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-xl);
		padding: var(--spacing-lg);
		cursor: pointer;
		text-align: left;
		transition: all 150ms ease-in-out;
	}

	.branch-option-button:hover {
		background-color: var(--bg-tertiary);
		border-color: var(--border-secondary);
	}

	.option-content {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}

	.option-content strong {
		color: var(--text-primary);
		font-size: var(--font-size-base);
		font-weight: 600;
	}

	.option-content span {
		color: var(--text-secondary);
		font-size: var(--font-size-sm);
	}

	.provider-sections {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
	}

	.provider-section h4 {
		margin: 0 0 var(--spacing-md) 0;
		font-size: var(--font-size-base);
		font-weight: 600;
		color: var(--text-primary);
	}

	.model-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
		gap: var(--spacing-sm);
	}

	.model-button {
		background: transparent;
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-sm);
		padding: var(--spacing-md);
		cursor: pointer;
		font-size: var(--font-size-sm);
		color: var(--text-primary);
		transition: all 150ms ease-in-out;
		text-align: center;
	}

	.model-button:hover {
		background-color: var(--bg-tertiary);
		border-color: var(--border-secondary);
	}

	.model-button.selected {
		background-color: var(--accent-primary);
		border-color: var(--accent-primary);
		color: white;
	}

	.modal-actions {
		display: flex;
		gap: var(--spacing-md);
		justify-content: flex-end;
		margin-top: var(--spacing-lg);
		padding-top: var(--spacing-lg);
		border-top: 1px solid var(--border-primary);
	}

	.modal-action-button {
		border: 1px solid var(--border-primary);
		border-radius: var(--radius-sm);
		padding: var(--spacing-sm) var(--spacing-lg);
		cursor: pointer;
		font-size: var(--font-size-sm);
		font-weight: 500;
		transition: all 150ms ease-in-out;
	}

	.modal-action-button.cancel {
		background: transparent;
		color: var(--text-primary);
	}

	.modal-action-button.cancel:hover {
		background-color: var(--bg-tertiary);
	}

	.modal-action-button.confirm {
		background-color: var(--accent-primary);
		border-color: var(--accent-primary);
		color: white;
	}

	.modal-action-button.confirm:hover:not(:disabled) {
		background-color: var(--accent-primary-dark, var(--accent-primary));
		opacity: 0.9;
	}

	.modal-action-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.user-content-wrapper.truncated {
		-webkit-mask-image: linear-gradient(
			to bottom,
			black 50%,
			transparent 100%
		);
		mask-image: linear-gradient(to bottom, black 50%, transparent 100%);
	}
</style>
