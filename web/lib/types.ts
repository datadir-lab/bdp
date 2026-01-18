// API Response Types
export interface ApiResponse<T> {
  data: T;
  status: number;
  success: boolean;
}

export interface ApiError {
  message: string;
  code: string;
  status: number;
  details?: Record<string, unknown>;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  limit: number;
  totalPages: number;
}

export type QueryParams = Record<string, string | number | boolean | undefined>;

// Blockchain Types
export interface Block {
  number: number;
  hash: string;
  parentHash: string;
  timestamp: number;
  miner: string;
  difficulty: string;
  totalDifficulty: string;
  size: number;
  gasUsed: string;
  gasLimit: string;
  transactionCount: number;
  extraData?: string;
}

export interface Transaction {
  hash: string;
  blockNumber: number;
  blockHash: string;
  from: string;
  to: string | null;
  value: string;
  gas: string;
  gasPrice: string;
  nonce: number;
  transactionIndex: number;
  input: string;
  timestamp: number;
  status?: number;
}

export interface Log {
  address: string;
  topics: string[];
  data: string;
  blockNumber: number;
  transactionHash: string;
  transactionIndex: number;
  blockHash: string;
  logIndex: number;
  removed: boolean;
}

export interface Event {
  id: string;
  blockNumber: number;
  transactionHash: string;
  logIndex: number;
  address: string;
  eventName: string;
  args: Record<string, unknown>;
  timestamp: number;
}

// Indexer Types
export interface Indexer {
  id: string;
  name: string;
  description: string;
  chain: string;
  contractAddress: string;
  abi: string;
  startBlock: number;
  currentBlock: number;
  status: 'active' | 'paused' | 'stopped' | 'error';
  createdAt: string;
  updatedAt: string;
}

export interface IndexerStatus {
  indexerId: string;
  currentBlock: number;
  latestBlock: number;
  syncPercentage: number;
  eventsIndexed: number;
  lastSyncAt: string;
  error?: string;
}

// Contract Types
export interface Contract {
  address: string;
  name?: string;
  abi: string;
  bytecode?: string;
  chain: string;
  createdAt: string;
  verified: boolean;
}

export interface ContractEvent {
  name: string;
  signature: string;
  inputs: {
    name: string;
    type: string;
    indexed: boolean;
  }[];
}

// Query Types
export interface QueryFilter {
  blockNumber?: number;
  fromBlock?: number;
  toBlock?: number;
  address?: string | string[];
  topics?: (string | string[] | null)[];
}

export interface TimeRangeFilter {
  startTime?: number;
  endTime?: number;
}

// Analytics Types
export interface BlockStats {
  totalBlocks: number;
  totalTransactions: number;
  totalGasUsed: string;
  averageBlockTime: number;
  averageGasPrice: string;
  period: 'hour' | 'day' | 'week' | 'month';
}

export interface TransactionStats {
  totalTransactions: number;
  successfulTransactions: number;
  failedTransactions: number;
  totalValue: string;
  averageGasPrice: string;
  period: 'hour' | 'day' | 'week' | 'month';
}

// User/Auth Types
export interface User {
  id: string;
  email: string;
  name?: string;
  apiKey?: string;
  createdAt: string;
  updatedAt: string;
}

export interface AuthTokens {
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
}

// Utility Types
export type SortOrder = 'asc' | 'desc';

export interface SortParams {
  sortBy: string;
  order: SortOrder;
}

export interface DateRange {
  from: Date;
  to: Date;
}

// Component Props Types
export interface BaseComponentProps {
  className?: string;
  children?: React.ReactNode;
}

export interface LoadingState {
  isLoading: boolean;
  error?: string;
}
