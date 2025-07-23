-- Add new columns to streaming_states table for enhanced functionality
ALTER TABLE streaming_states 
ADD COLUMN IF NOT EXISTS last_chunk_index INTEGER DEFAULT -1,
ADD COLUMN IF NOT EXISTS chunks JSONB DEFAULT '[]'::jsonb;

-- Create index on user_id and status for faster queries
CREATE INDEX IF NOT EXISTS idx_streaming_states_user_status 
ON streaming_states(user_id, status);

-- Create index on message_id for fast lookups
CREATE INDEX IF NOT EXISTS idx_streaming_states_message_id 
ON streaming_states(message_id);

-- Create index on created_at for cleanup queries
CREATE INDEX IF NOT EXISTS idx_streaming_states_created_at 
ON streaming_states(created_at);

-- Add a composite index for active stream lookups
CREATE INDEX IF NOT EXISTS idx_streaming_states_active 
ON streaming_states(user_id, status, created_at DESC) 
WHERE status = 'streaming';