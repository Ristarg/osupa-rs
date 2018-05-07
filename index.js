const rust = import("./osupa_rs");

function do_stuff(ev) {
    let f = ev.target.files[0];

    var reader = new FileReader();

    reader.onload = (function(file) {
        return function(e) {
            let bytes = new Uint8Array(e.target.result)
            rust.then(m => {
                alert('From JS:' + m.do_stuff(bytes))
            });
        }
    })(f);

    reader.readAsArrayBuffer(f)
}

document.getElementById('file').addEventListener('change', do_stuff, false)