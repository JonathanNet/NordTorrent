package com.nordtorrent.tv

data class User(
    val username: String,
    val displayName: String,
    val role: String,
    val createdAt: String,
)

data class Status(
    val setupRequired: Boolean,
    val version: String,
    val apiVersion: Int,
)

data class AuthResult(
    val user: User,
    val token: String,
)

data class LibraryItem(
    val torrentId: Int,
    val fileId: Int,
    val infoHash: String,
    val title: String,
    val fileName: String,
    val torrentName: String,
    val size: Long,
    val downloadedBytes: Long,
    val progress: Double,
    val kind: String,
    val mime: String,
    val streamUrl: String,
    val torrentState: String,
) {
    val stableId: Long = (torrentId.toLong() shl 32) or (fileId.toLong() and 0xffffffffL)
}

data class TorrentSummary(
    val id: Int,
    val infoHash: String,
    val name: String,
    val label: String = "",
    val state: String,
    val progress: Double,
    val progressBytes: Long,
    val totalBytes: Long,
    val uploadedBytes: Long,
    val downloadSpeedBps: Double,
    val uploadSpeedBps: Double,
    val etaSeconds: Long?,
    val ratio: Double,
    val finished: Boolean,
    val error: String?,
    val mediaFiles: Int,
    val totalFiles: Int,
)

data class Totals(
    val torrentCount: Int = 0,
    val active: Int = 0,
    val paused: Int = 0,
    val completed: Int = 0,
    val downloadSpeedBps: Double = 0.0,
    val uploadSpeedBps: Double = 0.0,
    val downloadedBytes: Long = 0,
    val uploadedBytes: Long = 0,
)

data class TorrentList(
    val torrents: List<TorrentSummary>,
    val totals: Totals,
)
