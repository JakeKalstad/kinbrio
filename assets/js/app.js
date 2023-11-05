function num_from_string(inp) {
    let n = parseInt(inp)
    if (isNaN(n)) {
        return 0
    }
    return n
}
function set_value(element, val) {
    if (element) {
        element.value = val;
    }
}
const dateForDateTimeInputValue = date => new Date(date.getTime() + new Date().getTimezoneOffset() * -60 * 1000).toISOString().slice(0, 19)

function send_delete(button_id, url, cb) {
    cb = cb || (() => { });
    var btn = document.getElementById(button_id);
    btn.onclick = function (event) {
        let confirmed = confirm(("Delete!?"))
        if (!confirmed) {
            return cb(false)
        }
        event.preventDefault();
        var xhr = new XMLHttpRequest(); 
        xhr.open('DELETE', url)
        xhr.onreadystatechange = function () {
            if (xhr.readyState == XMLHttpRequest.DONE) {
                cb(true, xhr.responseText)
            }
        }
        xhr.send(null)
        return false;
    }
}

function post_form(form_id, url, validation, cb) {
    validation = validation || (() => true);
    cb = cb || (() => { });

    var form = document.getElementById(form_id);
    form.onsubmit = function (event) {
        event.preventDefault();
        var xhr = new XMLHttpRequest();
        var formData = new FormData(form);
        formData = validation(Object.fromEntries(formData));
        if (!formData) {
            return
        }
        xhr.open('POST', url)
        xhr.setRequestHeader("Content-Type", form.enctype||"application/json");
        xhr.send(JSON.stringify(formData));
        xhr.onreadystatechange = function () {
            if (xhr.readyState == XMLHttpRequest.DONE) {
                cb(xhr.responseText)
            }
        }
        return false;
    }
}

function deleteAllCookies() {
    var cookies = document.cookie.split(";");
    for (var i = 0; i < cookies.length; i++) {
        var cookie = cookies[i];
        var eqPos = cookie.indexOf("=");
        var name = eqPos > -1 ? cookie.substr(0, eqPos) : cookie;
        document.cookie = name + "=;expires=Thu, 01 Jan 1970 00:00:00 GMT";
    }
}

function clearAndRedirect(link) {
    deleteAllCookies();
    document.location = link;
}
