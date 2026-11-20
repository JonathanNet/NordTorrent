package com.nordtorrent.tv

import android.content.Intent
import android.os.Bundle
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.core.view.isVisible
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import androidx.media3.ui.PlayerView

class PlayerActivity : AppCompatActivity() {
    private lateinit var playerView: PlayerView
    private lateinit var titleView: TextView
    private lateinit var errorView: TextView
    private var player: ExoPlayer? = null
    private var mediaUrl: String = ""
    private var resumePosition: Long = 0

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        mediaUrl = intent.getStringExtra(EXTRA_URL).orEmpty()
        val title = intent.getStringExtra(EXTRA_TITLE).orEmpty()
        val token = SessionStore(this).token()
        if (mediaUrl.isBlank() || token.isNullOrBlank()) {
            finish()
            return
        }

        setContentView(R.layout.activity_player)
        playerView = findViewById(R.id.player_view)
        titleView = findViewById(R.id.player_title)
        errorView = findViewById(R.id.player_error)
        titleView.text = title.ifBlank { getString(R.string.player_title) }
        findViewById<TextView>(R.id.player_back).setOnClickListener { finish() }

        hideSystemUi()
        resumePosition = getSharedPreferences(PLAYBACK_PREFS, MODE_PRIVATE)
            .getLong(positionKey(mediaUrl), 0L)

        val dataSourceFactory = DefaultHttpDataSource.Factory()
            .setDefaultRequestProperties(mapOf("Authorization" to "Bearer $token"))
            .setConnectTimeoutMs(15_000)
            .setReadTimeoutMs(30_000)
            .setAllowCrossProtocolRedirects(false)

        val exoPlayer = ExoPlayer.Builder(this)
            .setMediaSourceFactory(DefaultMediaSourceFactory(dataSourceFactory))
            .build()
        player = exoPlayer
        playerView.player = exoPlayer
        playerView.requestFocus()

        exoPlayer.addListener(
            object : Player.Listener {
                override fun onPlayerError(error: PlaybackException) {
                    errorView.isVisible = true
                }

                override fun onPlaybackStateChanged(playbackState: Int) {
                    if (playbackState == Player.STATE_READY) errorView.isVisible = false
                }
            },
        )

        exoPlayer.setMediaItem(MediaItem.fromUri(mediaUrl))
        if (resumePosition > 10_000) exoPlayer.seekTo(resumePosition)
        exoPlayer.prepare()
        exoPlayer.playWhenReady = true
    }

    override fun onPause() {
        savePosition()
        super.onPause()
    }

    override fun onDestroy() {
        player?.release()
        player = null
        super.onDestroy()
    }

    private fun savePosition() {
        val current = player ?: return
        val duration = current.duration
        val position = current.currentPosition
        if (duration > 0 && position in 10_000 until (duration - 15_000)) {
            getSharedPreferences(PLAYBACK_PREFS, MODE_PRIVATE)
                .edit()
                .putLong(positionKey(mediaUrl), position)
                .apply()
        } else if (duration > 0 && position >= duration - 15_000) {
            getSharedPreferences(PLAYBACK_PREFS, MODE_PRIVATE)
                .edit()
                .remove(positionKey(mediaUrl))
                .apply()
        }
    }

    private fun hideSystemUi() {
        WindowCompat.setDecorFitsSystemWindows(window, false)
        WindowInsetsControllerCompat(window, window.decorView).apply {
            hide(WindowInsetsCompat.Type.systemBars())
            systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
    }

    private fun positionKey(url: String): String = "position_${url.hashCode()}"

    companion object {
        const val EXTRA_URL = "stream_url"
        const val EXTRA_TITLE = "stream_title"
        private const val PLAYBACK_PREFS = "nordtorrent_playback"
    }
}
