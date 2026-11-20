package com.nordtorrent.tv

import android.content.Context
import java.util.Locale
import kotlin.math.roundToLong

fun formatBytes(value: Long): String {
    val number = value.coerceAtLeast(0)
    if (number < 1024) return "$number B"
    val units = arrayOf("KB", "MB", "GB", "TB", "PB")
    var size = number.toDouble() / 1024.0
    var index = 0
    while (size >= 1024.0 && index < units.lastIndex) {
        size /= 1024.0
        index += 1
    }
    val digits = when {
        size >= 100 -> 0
        size >= 10 -> 1
        else -> 2
    }
    return String.format(Locale.getDefault(), "%.${digits}f %s", size, units[index])
}

fun formatSpeed(bytesPerSecond: Double): String {
    return "${formatBytes(bytesPerSecond.roundToLong())}/s"
}

fun formatEta(seconds: Long?): String {
    if (seconds == null || seconds <= 0) return "–"
    val total = seconds
    val hours = total / 3600
    val minutes = (total % 3600) / 60
    val secs = total % 60
    return when {
        hours > 0 -> "${hours}h ${minutes}m"
        minutes > 0 -> "${minutes}m ${secs}s"
        else -> "${secs}s"
    }
}

fun torrentStateLabel(context: Context, torrent: TorrentSummary): String = when (torrent.state) {
    "downloading" -> context.getString(R.string.state_downloading)
    "seeding" -> context.getString(R.string.state_seeding)
    "paused" -> context.getString(R.string.state_paused)
    "initializing" -> context.getString(R.string.state_initializing)
    "error" -> context.getString(R.string.state_error)
    else -> context.getString(
        if (torrent.finished) R.string.state_finished else R.string.state_ready,
    )
}

fun torrentStateColor(torrent: TorrentSummary): Int = when (torrent.state) {
    "downloading" -> 0xFFC6F45C.toInt()
    "seeding" -> 0xFF7DD3FC.toInt()
    "error" -> 0xFFFB7185.toInt()
    "initializing" -> 0xFFFBBF24.toInt()
    else -> 0xFF8D9AA3.toInt()
}

fun progressPercent(value: Double): Int = value.coerceIn(0.0, 100.0).roundToLong().toInt()

fun roleLabel(context: Context, role: String): String = context.getString(
    if (role == "admin") R.string.role_admin_label else R.string.role_viewer_label,
)
