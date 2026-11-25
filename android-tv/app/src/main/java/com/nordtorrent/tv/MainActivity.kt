package com.nordtorrent.tv

import android.content.Intent
import android.os.Bundle
import android.widget.Button
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.isVisible
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.GridLayoutManager
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

class MainActivity : AppCompatActivity() {
    private enum class Page { LIBRARY, TRANSFERS, SETTINGS }

    private lateinit var store: SessionStore
    private lateinit var api: ApiClient
    private lateinit var libraryAdapter: MediaAdapter
    private lateinit var transferAdapter: TransferAdapter

    private lateinit var libraryPage: LinearLayout
    private lateinit var transfersPage: LinearLayout
    private lateinit var settingsPage: LinearLayout
    private lateinit var libraryList: RecyclerView
    private lateinit var transfersList: RecyclerView
    private lateinit var libraryEmpty: TextView
    private lateinit var transfersEmpty: TextView
    private lateinit var loadingOverlay: LinearLayout
    private lateinit var libraryCount: TextView
    private lateinit var navLibrary: Button
    private lateinit var navTransfers: Button
    private lateinit var navSettings: Button
    private lateinit var serverLabel: TextView
    private lateinit var userAvatar: TextView
    private lateinit var settingsUser: TextView
    private lateinit var settingsRole: TextView
    private lateinit var settingsServer: TextView
    private lateinit var settingsApi: TextView
    private lateinit var settingsVersion: TextView
    private var statDownload: TextView? = null
    private var statUpload: TextView? = null
    private var statUploadSub: TextView? = null
    private var statActive: TextView? = null
    private var statActiveSub: TextView? = null
    private var statDownloaded: TextView? = null
    private var statDownloadedSub: TextView? = null

    private var currentPage = Page.LIBRARY
    private var loading = false
    private var pollJob: Job? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        store = SessionStore(this)
        val baseUrl = store.baseUrl()
        val token = store.token()
        if (baseUrl.isBlank() || token.isNullOrBlank()) {
            goToLogin()
            return
        }
        api = ApiClient(baseUrl, token, context = this)
        setContentView(R.layout.activity_main)
        applySystemBarInsets(this)
        bindViews()
        bindData()
        setupLists()
        setupNavigation()
        updateStaticDetails()
        selectPage(Page.LIBRARY)
        loadData(showLoading = true)

        onBackPressedDispatcher.addCallback(
            this,
            object : OnBackPressedCallback(true) {
                override fun handleOnBackPressed() {
                    if (currentPage != Page.LIBRARY) {
                        selectPage(Page.LIBRARY)
                    } else {
                        isEnabled = false
                        onBackPressedDispatcher.onBackPressed()
                    }
                }
            },
        )
    }

    override fun onStart() {
        super.onStart()
        if (::api.isInitialized) startPolling()
    }

    override fun onStop() {
        pollJob?.cancel()
        pollJob = null
        super.onStop()
    }

    private fun bindViews() {
        libraryPage = findViewById(R.id.library_page)
        transfersPage = findViewById(R.id.transfers_page)
        settingsPage = findViewById(R.id.settings_page)
        libraryList = findViewById(R.id.library_list)
        transfersList = findViewById(R.id.transfers_list)
        libraryEmpty = findViewById(R.id.library_empty)
        transfersEmpty = findViewById(R.id.transfers_empty)
        loadingOverlay = findViewById(R.id.loading_overlay)
        libraryCount = findViewById(R.id.library_count)
        navLibrary = findViewById(R.id.nav_library)
        navTransfers = findViewById(R.id.nav_transfers)
        navSettings = findViewById(R.id.nav_settings)
        serverLabel = findViewById(R.id.server_label)
        userAvatar = findViewById(R.id.user_avatar)
        settingsUser = findViewById(R.id.settings_user)
        settingsRole = findViewById(R.id.settings_role)
        settingsServer = findViewById(R.id.settings_server)
        settingsApi = findViewById(R.id.settings_api)
        settingsVersion = findViewById(R.id.settings_version)
        statDownload = findViewById<TextView>(R.id.stat_download)
        statUpload = findViewById<TextView>(R.id.stat_upload)
        statUploadSub = findViewById<TextView>(R.id.stat_upload_sub)
        statActive = findViewById<TextView>(R.id.stat_active)
        statActiveSub = findViewById<TextView>(R.id.stat_active_sub)
        statDownloaded = findViewById<TextView>(R.id.stat_downloaded)
        statDownloadedSub = findViewById<TextView>(R.id.stat_downloaded_sub)
    }

    private fun bindData() {
        libraryAdapter = MediaAdapter { item -> play(item) }
        transferAdapter = TransferAdapter { torrent -> transferActions(torrent) }
    }

    private fun setupLists() {
        libraryList.layoutManager = GridLayoutManager(this, gridColumns())
        libraryList.adapter = libraryAdapter
        libraryList.addItemDecoration(GridSpacingDecoration(resources.getDimensionPixelSize(R.dimen.card_spacing)))
        libraryList.setHasFixedSize(false)

        transfersList.layoutManager = LinearLayoutManager(this)
        transfersList.adapter = transferAdapter
        transfersList.setHasFixedSize(false)
    }

    private fun updateStats(totals: Totals) {
        statDownload?.text = formatSpeed(totals.downloadSpeedBps)
        statUpload?.text = formatSpeed(totals.uploadSpeedBps)
        statActive?.text = totals.active.toString()
        statActiveSub?.text = resources.getQuantityString(
            R.plurals.stat_torrents,
            totals.torrentCount,
            totals.torrentCount,
        )
        statDownloaded?.text = formatBytes(totals.downloadedBytes)
        statUploadSub?.text = formatBytes(totals.uploadedBytes)
        statDownloadedSub?.text = ""
    }

    private fun gridColumns(): Int {
        val widthDp = resources.configuration.screenWidthDp
        return when {
            widthDp >= 1000 -> 4
            widthDp >= 700 -> 3
            else -> 2
        }
    }

    private fun setupNavigation() {
        navLibrary.setOnClickListener { selectPage(Page.LIBRARY) }
        navTransfers.setOnClickListener { selectPage(Page.TRANSFERS) }
        navSettings.setOnClickListener { selectPage(Page.SETTINGS) }
        findViewById<ImageButton>(R.id.refresh_button).setOnClickListener { loadData(true) }
        findViewById<Button>(R.id.logout_button).setOnClickListener { confirmLogout() }
    }

    private fun updateStaticDetails() {
        val display = store.displayName().ifBlank { store.username() }
        userAvatar.text = display.trim().take(1).uppercase().ifBlank { "N" }
        serverLabel.text = store.baseUrl()
            .removePrefix("https://")
            .removePrefix("http://")
        settingsUser.text = display
        settingsRole.text = roleLabel(this, store.role())
        settingsServer.text = store.baseUrl()
        settingsApi.text = "Bearer-token · API v1"
        settingsVersion.text = "NordTorrent TV ${BuildConfig.VERSION_NAME}"
    }

    private fun selectPage(page: Page) {
        currentPage = page
        libraryPage.isVisible = page == Page.LIBRARY
        transfersPage.isVisible = page == Page.TRANSFERS
        settingsPage.isVisible = page == Page.SETTINGS
        navLibrary.isSelected = page == Page.LIBRARY
        navTransfers.isSelected = page == Page.TRANSFERS
        navSettings.isSelected = page == Page.SETTINGS
        when (page) {
            Page.LIBRARY -> focusLibrary()
            Page.TRANSFERS -> {
                if (transferAdapter.itemCount > 0) transfersList.requestFocus()
                else navTransfers.requestFocus()
            }
            Page.SETTINGS -> navSettings.requestFocus()
        }
    }

    private fun focusLibrary() {
        libraryList.postDelayed({
            if (currentPage != Page.LIBRARY) return@postDelayed
            val holder = libraryList.findViewHolderForAdapterPosition(0)
            if (holder != null) holder.itemView.requestFocus()
            else navLibrary.requestFocus()
        }, 120)
    }

    private fun startPolling() {
        pollJob?.cancel()
        pollJob = lifecycleScope.launch {
            while (isActive) {
                delay(6_000)
                if (!loading && !isFinishing) loadData(showLoading = false)
            }
        }
    }

    private fun loadData(showLoading: Boolean) {
        if (loading || !::api.isInitialized) return
        loading = true
        if (showLoading) loadingOverlay.isVisible = true
        lifecycleScope.launch {
            try {
                val library = api.library()
                val torrents = api.torrents()
                libraryAdapter.submitList(library)
                transferAdapter.submitList(torrents.torrents)
                libraryCount.text = library.size.toString()
                updateStats(torrents.totals)
                libraryEmpty.isVisible = library.isEmpty()
                libraryList.isVisible = library.isNotEmpty()
                transfersEmpty.isVisible = torrents.torrents.isEmpty()
                transfersList.isVisible = torrents.torrents.isNotEmpty()
                if (currentPage == Page.LIBRARY && library.isNotEmpty()) focusLibrary()
            } catch (error: ApiException) {
                if (error.statusCode == 401) {
                    sessionExpired(error.message ?: getString(R.string.auth_expired))
                } else {
                    toast(error.message ?: getString(R.string.error_generic))
                }
            } catch (error: Exception) {
                toast(error.message ?: getString(R.string.connection_error))
            } finally {
                loading = false
                loadingOverlay.isVisible = false
            }
        }
    }

    private fun play(item: LibraryItem) {
        startActivity(
            Intent(this, PlayerActivity::class.java)
                .putExtra(PlayerActivity.EXTRA_URL, api.streamUrl(item.streamUrl))
                .putExtra(PlayerActivity.EXTRA_TITLE, item.title.ifBlank { item.fileName }),
        )
    }

    private fun transferActions(torrent: TorrentSummary) {
        if (store.role() != "admin") {
            toast(getString(R.string.transfer_no_access))
            return
        }
        val isPaused = torrent.state == "paused"
        val actionLabel = if (isPaused) getString(R.string.transfer_resume) else getString(R.string.transfer_pause)
        AlertDialog.Builder(this)
            .setTitle(torrent.name.ifBlank { "Torrent" })
            .setItems(arrayOf(actionLabel, getString(R.string.transfer_cancel))) { _, which ->
                if (which == 0) {
                    lifecycleScope.launch {
                        try {
                            if (isPaused) api.resumeTorrent(torrent.id) else api.pauseTorrent(torrent.id)
                            toast(
                                getString(
                                    if (isPaused) R.string.transfer_resumed_ok else R.string.transfer_paused_ok,
                                ),
                            )
                            loadData(showLoading = false)
                        } catch (error: ApiException) {
                            toast(error.message ?: getString(R.string.error_generic))
                        } catch (error: Exception) {
                            toast(error.message ?: getString(R.string.error_generic))
                        }
                    }
                }
            }
            .show()
    }

    private fun confirmLogout() {
        AlertDialog.Builder(this)
            .setTitle(R.string.logout_confirm_title)
            .setMessage(R.string.logout_confirm_text)
            .setNegativeButton(R.string.logout_no, null)
            .setPositiveButton(R.string.logout_yes) { _, _ ->
                lifecycleScope.launch {
                    api.logout()
                    store.clear()
                    goToLogin()
                }
            }
            .show()
    }

    private fun sessionExpired(message: String) {
        if (isFinishing) return
        store.clear()
        toast(message)
        goToLogin()
    }

    private fun goToLogin() {
        startActivity(
            Intent(this, LoginActivity::class.java).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
            },
        )
        finish()
    }

    private fun toast(message: String) {
        Toast.makeText(this, message, Toast.LENGTH_LONG).show()
    }
}
