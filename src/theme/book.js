$( document ).ready(function() {

    // url
    var url = window.location.pathname;

    // Fix back button cache problem
    window.onunload = function(){};

    // Set theme
    var theme = store.get('mdbook-theme');
    if (theme === null || theme === undefined) { theme = 'light'; }

    set_theme(theme);

    // Syntax highlighting Configuration
    hljs.configure({
        tabReplace: '    ', // 4 spaces
        languages: [],      // Languages used for auto-detection
    });

    if (window.ace) {
        // language-rust class needs to be removed for editable
        // blocks or highlightjs will capture events
        $('code.editable').removeClass('language-rust');

        $('code').not('.editable').each(function(i, block) {
            hljs.highlightBlock(block);
        });
    } else {
        $('code').each(function(i, block) {
            hljs.highlightBlock(block);
        });
    }

    // Adding the hljs class gives code blocks the color css
    // even if highlighting doesn't apply
    $('code').addClass('hljs');

    var KEY_CODES = {
        PREVIOUS_KEY: 37,
        NEXT_KEY: 39,
        ESCAPE_KEY: 27,
    };

    $(document).on('keydown', function (e) {
        if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) { return; }
        switch (e.keyCode) {
            case KEY_CODES.NEXT_KEY:
                e.preventDefault();
                if($('.nav-chapters.next').length) {
                    window.location.href = $('.nav-chapters.next').attr('href');
                }
                break;
            case KEY_CODES.PREVIOUS_KEY:
                e.preventDefault();
                if($('.nav-chapters.previous').length) {
                    window.location.href = $('.nav-chapters.previous').attr('href');
                }
                break;
            case KEY_CODES.ESCAPE_KEY:
                e.preventDefault();
                hideThemes();
                break;
        }
    });

    // Interesting DOM Elements
    var sidebar = $("#sidebar");

    // Toggle sidebar
    $("#sidebar-toggle").click(sidebarToggle);

    // Scroll sidebar to current active section
    var activeSection = sidebar.find(".active");
    if(activeSection.length) {
        sidebar.scrollTop(activeSection.offset().top);
    }

    var firstContact = null;

    $(this).on('touchstart', function(e) {
        firstContact = {
            x: e.originalEvent.touches[0].clientX,
            time: Date.now()
        };
    });
    
    $(this).on('touchmove', function(e) {
        if (!firstContact) 
            return;

        var curX = e.originalEvent.touches[0].clientX;
        var xDiff = curX - firstContact.x,
            tDiff = Date.now() - firstContact.time;

        if (tDiff < 250 && Math.abs(xDiff) >= 150) {
            if (xDiff >= 0 && firstContact.x < Math.min(document.body.clientWidth * 0.25, 300))
                showSidebar();
            else if (xDiff < 0 && curX < 300)
                hideSidebar();

            firstContact = null;
        }
    });

    function showThemes() {
        $('.theme-popup').css('display', 'block');
        $('#theme-toggle').attr('aria-expanded', true);
    }

    function hideThemes() {
        $('.theme-popup').css('display', 'none');
        $('#theme-toggle').attr('aria-expanded', false);
    }

    // Theme button
    $("#theme-toggle").click(function(){
        if ($('.theme-popup').css('display') === 'block') {
            hideThemes();
        } else {
            showThemes();
        }
    });

    $('.theme').click(function(){
        var theme = $(this).attr('id');
        set_theme(theme);
    });

    // Hide theme selector popup when clicking outside of it
    $(document).click(function(event){
        var popup = $('.theme-popup');
        if(popup.css('display') === 'block') {
            var target = $(event.target);
            if(!target.closest('.theme').length && !target.closest('#theme-toggle').length) {
                hideThemes();
            }
        }
    });

    function set_theme(theme) {
        let ace_theme;

        if (theme == 'coal' || theme == 'navy') {
            $("[href='ayu-highlight.css']").prop('disabled', true);
            $("[href='tomorrow-night.css']").prop('disabled', false);
            $("[href='highlight.css']").prop('disabled', true);

            ace_theme = "ace/theme/tomorrow_night";
        } else if (theme == 'ayu') {
            $("[href='ayu-highlight.css']").prop('disabled', false);
            $("[href='tomorrow-night.css']").prop('disabled', true);
            $("[href='highlight.css']").prop('disabled', true);

            ace_theme = "ace/theme/tomorrow_night";
        } else {
            $("[href='ayu-highlight.css']").prop('disabled', true);
            $("[href='tomorrow-night.css']").prop('disabled', true);
            $("[href='highlight.css']").prop('disabled', false);

            ace_theme = "ace/theme/dawn";
        }

        if (window.ace && window.editors) {
            window.editors.forEach(function(editor) {
                editor.setTheme(ace_theme);
            });
        }

        store.set('mdbook-theme', theme);

        $('body').removeClass().addClass(theme);
    }


    // Hide Rust code lines prepended with a specific character
    var hiding_character = "#";

    $("code.language-rust").each(function(i, block){

        var code_block = $(this);
        var pre_block = $(this).parent();
        // hide lines
        var lines = code_block.html().split("\n");
        var first_non_hidden_line = false;
        var lines_hidden = false;

        for(var n = 0; n < lines.length; n++){
            if($.trim(lines[n])[0] == hiding_character){
                if(first_non_hidden_line){
                    lines[n] = "<span class=\"hidden\">" + "\n" + lines[n].replace(/(\s*)# ?/, "$1") + "</span>";
                }
                else {
                    lines[n] = "<span class=\"hidden\">" + lines[n].replace(/(\s*)# ?/, "$1") + "\n"  +  "</span>";
                }
                lines_hidden = true;
            }
            else if(first_non_hidden_line) {
                lines[n] = "\n" + lines[n];
            }
            else {
                first_non_hidden_line = true;
            }
        }
        code_block.html(lines.join(""));

        // If no lines were hidden, return
        if(!lines_hidden) { return; }

        // add expand button
        pre_block.prepend("<div class=\"buttons\"><i class=\"fa fa-expand\" title=\"Show hidden lines\"></i></div>");

        pre_block.find("i").click(function(e){
            if( $(this).hasClass("fa-expand") ) {
                $(this).removeClass("fa-expand").addClass("fa-compress");
                $(this).attr("title", "Hide lines");
                pre_block.find("span.hidden").removeClass("hidden").addClass("unhidden");
            }
            else {
                $(this).removeClass("fa-compress").addClass("fa-expand");
                $(this).attr("title", "Show hidden lines");
                pre_block.find("span.unhidden").removeClass("unhidden").addClass("hidden");
            }
        });
    });
    
    $("pre code").each(function(i, block){
        var pre_block = $(this).parent();
        if( !pre_block.hasClass('playpen') ) {
            var buttons = pre_block.find(".buttons");
            if(buttons.length == 0) {
                pre_block.prepend("<div class=\"buttons\"></div>");
                buttons = pre_block.find(".buttons");
            }
            buttons.prepend("<i class=\"fa fa-copy clip-button\"><i class=\"tooltiptext\"></i></i>");
            buttons.find(".clip-button").mouseout(function(e){
                hideTooltip(e.currentTarget);
            });
        }
    });

    // Process playpen code blocks
    $(".playpen").each(function(block){
        var pre_block = $(this);
        // Add play button
        var buttons = pre_block.find(".buttons");
        if( buttons.length === 0 ) {
            pre_block.prepend("<div class=\"buttons\"></div>");
            buttons = pre_block.find(".buttons");
        }
        buttons.prepend("<i class=\"fa fa-play play-button hidden\" title=\"Run this code\"></i>");
        buttons.prepend("<i class=\"fa fa-copy clip-button\" title=\"Copy to clipboard\"><i class=\"tooltiptext\"></i></i>");

        let code_block = pre_block.find("code").first();
        if (window.ace && code_block.hasClass("editable")) {
            buttons.prepend("<i class=\"fa fa-history reset-button\" title=\"Undo changes\"></i>");
        }

        buttons.find(".play-button").click(function(e){
            run_rust_code(pre_block);
        });
        buttons.find(".clip-button").mouseout(function(e){
            hideTooltip(e.currentTarget);
        });
        buttons.find(".reset-button").click(function() {
            if (!window.ace) { return; }
            let editor = window.ace.edit(code_block.get(0));
            editor.setValue(editor.originalCode);
            editor.clearSelection();
        });
    });

    var clipboardSnippets = new Clipboard('.clip-button', {
        text: function(trigger) {
            hideTooltip(trigger);
            let playpen = $(trigger).parents("pre");
            return playpen_text(playpen);
        }
    });
    clipboardSnippets.on('success', function(e) {
            e.clearSelection();
            showTooltip(e.trigger, "Copied!");
    });
    clipboardSnippets.on('error', function(e) {
            showTooltip(e.trigger, "Clipboard error!");
    });

    $.ajax({
        url: "https://play.rust-lang.org/meta/crates",
        method: "POST",
        crossDomain: true,
        dataType: "json",
        contentType: "application/json",
        success: function(response){
            // get list of crates available in the rust playground
            let playground_crates = response.crates.map(function(item) {return item["id"];} );
            $(".playpen").each(function(block) {
                handle_crate_list_update($(this), playground_crates);
            });
        },
    });

});

function playpen_text(playpen) {
    let code_block = playpen.find("code").first();

    if (window.ace && code_block.hasClass("editable")) {
        let editor = window.ace.edit(code_block.get(0));
        return editor.getValue();
    } else {
        return code_block.get(0).textContent;
    }
}

function handle_crate_list_update(playpen_block, playground_crates) {
    // update the play buttons after receiving the response
    update_play_button(playpen_block, playground_crates);

    // and install on change listener to dynamically update ACE editors
    if (window.ace) {
        let code_block = playpen_block.find("code").first();
        if (code_block.hasClass("editable")) {
            let editor = window.ace.edit(code_block.get(0));
            editor.on("change", function(e){
                update_play_button(playpen_block, playground_crates);
            });
        }
    }
}

// updates the visibility of play button based on `no_run` class and
// used crates vs ones available on http://play.rust-lang.org
function update_play_button(pre_block, playground_crates) {
    var play_button = pre_block.find(".play-button");

    var classes = pre_block.find("code").attr("class").split(" ");
    // skip if code is `no_run`
    if (classes.indexOf("no_run") > -1) {
        play_button.addClass("hidden");
        return;
    }

    // get list of `extern crate`'s from snippet
    var txt = playpen_text(pre_block);
    var re = /extern\s+crate\s+([a-zA-Z_0-9]+)\s*;/g;
    var snippet_crates = [];
    while (item = re.exec(txt)) {
        snippet_crates.push(item[1]);
    }

    // check if all used crates are available on play.rust-lang.org
    var all_available = snippet_crates.every(function(elem) {
        return playground_crates.indexOf(elem) > -1;
    });

    if (all_available) {
        play_button.removeClass("hidden");
    } else {
        play_button.addClass("hidden");
    }
}

function hideTooltip(elem) {
    elem.firstChild.innerText="";
    elem.setAttribute('class', 'fa fa-copy clip-button');
}

function showTooltip(elem, msg) {
    elem.firstChild.innerText=msg;
    elem.setAttribute('class', 'fa fa-copy tooltipped');
}

function sidebarToggle() {
    var html = $("html");
    if (html.hasClass("sidebar-hidden")) {
        showSidebar();
    } else if (html.hasClass("sidebar-visible")) {
        hideSidebar();
    } else {
        if ($("#sidebar").position().left === 0){
            hideSidebar();
        } else {
            showSidebar();
        }
    }
}

function showSidebar() {
    $('html').removeClass('sidebar-hidden').addClass('sidebar-visible');
    $('#sidebar a').attr('tabIndex', 0);
    $('#sidebar-toggle').attr('aria-expanded', true);
    $('#sidebar').attr('aria-hidden', false);
    store.set('mdbook-sidebar', 'visible');
}

function hideSidebar() {
    $('html').removeClass('sidebar-visible').addClass('sidebar-hidden');
    $('#sidebar a').attr('tabIndex', -1);
    $('#sidebar-toggle').attr('aria-expanded', false);
    $('#sidebar').attr('aria-hidden', true);
    store.set('mdbook-sidebar', 'hidden');
}

function run_rust_code(code_block) {
    var result_block = code_block.find(".result");
    if(result_block.length === 0) {
        code_block.append("<code class=\"result hljs language-bash\"></code>");
        result_block = code_block.find(".result");
    }

    let text = playpen_text(code_block);
    
    var params = {
	channel: "stable",
	mode: "debug",
	crateType: "bin",
	tests: false,
	code: text,
    }

    if(text.indexOf("#![feature") !== -1) {
        params.channel = "nightly";
    }

    result_block.text("Running...");

    $.ajax({
        url: "https://play.rust-lang.org/execute",
        method: "POST",
        crossDomain: true,
        dataType: "json",
        contentType: "application/json",
        data: JSON.stringify(params),
        timeout: 15000,
        success: function(response){
           result_block.text(response.success ? response.stdout : response.stderr);
        },
        error: function(qXHR, textStatus, errorThrown){
            result_block.text("Playground communication " + textStatus);
        },
    });
}
