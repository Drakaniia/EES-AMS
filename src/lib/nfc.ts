// USB NFC Card Reader helpers. Works with external USB NFC card readers (13.56MHz IC Card Reader).
// Uses Tauri backend commands for native USB device communication.

import { invoke } from '@tauri-apps/api/core';

export type NfcSupport = 'connected' | 'disconnected';

export interface NfcReaderStatus {
	connected: boolean;
	readerName?: string;
	error?: string;
}

export interface NfcCardData {
	serialNumber: string;
	data?: string;
}

export async function nfcSupported(): Promise<NfcSupport> {
	if (typeof window === 'undefined') return 'disconnected';
	try {
		const status: NfcReaderStatus = await invoke('check_nfc_reader');
		return status.connected ? 'connected' : 'disconnected';
	} catch {
		return 'disconnected';
	}
}

export class NfcScanner {
	private onRead: (serial: string) => void;
	private onError: (err: Error) => void;
	private scanning = false;
	private scanInterval: ReturnType<typeof setInterval> | null = null;

	constructor(onRead: (serial: string) => void, onError: (err: Error) => void) {
		this.onRead = onRead;
		this.onError = onError;
	}

	async start() {
		if (this.scanning) return;

		try {
			// Check if NFC reader is available
			const status: NfcReaderStatus = await invoke('check_nfc_reader');
			if (!status.connected) {
				this.onError(new Error(status.error || 'NFC Card Reader not connected.'));
				return;
			}

			// Start NFC scanning in backend
			await invoke('start_nfc_scanning');
			this.scanning = true;

			// Start polling for card reads
			this.scanInterval = setInterval(async () => {
				if (!this.scanning) return;

				try {
					const cardData: NfcCardData = await invoke('read_nfc_card');
					if (cardData.serialNumber) {
						this.onRead(cardData.serialNumber);
					}
				} catch (error) {
					// Ignore "no card detected" errors during normal polling
					if (error instanceof Error && !error.message.includes('No card detected')) {
						this.onError(error);
					}
				}
			}, 1000); // Poll every second
		} catch (e: unknown) {
			this.onError(e instanceof Error ? e : new Error(String(e)));
		}
	}

	stop() {
		if (!this.scanning) return;

		this.scanning = false;
		if (this.scanInterval) {
			clearInterval(this.scanInterval);
			this.scanInterval = null;
		}

		// Stop NFC scanning in backend
		invoke('stop_nfc_scanning').catch(console.error);
	}
}
